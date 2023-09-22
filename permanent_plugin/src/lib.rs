use std::mem;
use std::fs::File;
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;
use core::ffi;
use lazy_static::lazy_static;
use iced_x86::{Decoder, DecoderOptions, Mnemonic};
use crossbeam_channel::Sender;
use enumset::EnumSet;

use permanent_common::config::{TraceOption, TcgPluginConfig};
use permanent_common::trace::{PmemEvent, new_trace_writer_bin};

mod qemu_plugin_bindings;
use qemu_plugin_bindings as qp;

mod writer;
use writer::{TraceMessage, writer_main};

#[no_mangle]
pub static qemu_plugin_version: ffi::c_int = qp::QEMU_PLUGIN_VERSION as ffi::c_int;
#[no_mangle]
pub static permanent_trace_version: ffi::c_int = 1;

//------------------------------------------------------------------------------

#[derive(Debug)]
enum UserdataMem {
    ReadWrite { disas: String, nt: bool },
    Clflush { disas: String },
    Clflushopt { disas: String },
    Clwb { disas: String },
    Checkpoint,
}

#[derive(Debug)]
enum UserdataExec {
    Wbinvd { disas: String },
    Fence { disas: String },
}

struct WriterThread {
    trace_send: Sender<TraceMessage>,
    done_send: Sender<()>,
    handle: JoinHandle<()>,
}


// save all userdata to make sure the callbacks don't do use-after-free
// use Box here so that the address doesn't change on vector insertion/reallocation
lazy_static! {
    static ref USERDATA_MEM_VEC: Mutex<Vec<Box<UserdataMem>>> = Mutex::new(Vec::new());
    static ref USERDATA_EXEC_VEC: Mutex<Vec<Box<UserdataExec>>> = Mutex::new(Vec::new());
    // static ref HAVE_WRITES: Mutex<bool> = Mutex::new(false);
    static ref HAVE_CHECKPOINT_SIGNAL_INSN: Mutex<bool> = Mutex::new(false);
    static ref HAVE_PMEM_INIT: Mutex<bool> = Mutex::new(false);
    
    static ref WRITER_THREAD: Mutex<Option<WriterThread>> = Mutex::new(None);
}
static CONFIG: OnceLock<TcgPluginConfig> = OnceLock::new();

fn get_conf() -> &'static TcgPluginConfig {
    &CONFIG.get().unwrap()
}

fn send_msg(msg: TraceMessage) {
    WRITER_THREAD.lock().unwrap()
            .as_mut().unwrap()
            .trace_send.send(msg)
            .expect("failed send to writer thread");
}

#[no_mangle]
extern "C" fn my_vcpu_insn_exec_cb(_vcpu_index: ffi::c_uint, userdata: *mut ffi::c_void) {
    let u: &UserdataExec = unsafe { &*(userdata as *const UserdataExec) };
    
    // filtering happens in hook_insn
    match u {
        UserdataExec::Wbinvd { .. } => {
            // *HAVE_WRITES.lock().unwrap() = true;
            send_msg(TraceMessage::Pmem(PmemEvent::Wbinvd));
        },
        UserdataExec::Fence { .. } => {
            // let mut have_writes = HAVE_WRITES.lock().unwrap();
            // if *have_writes {
            //     *have_writes = false;
            //     send_msg(TraceMessage::Pmem(PmemEvent::Fence));
            // }
            send_msg(TraceMessage::Pmem(PmemEvent::Fence));
        },
    }
}

#[no_mangle]
extern "C" fn my_vcpu_mem_cb(vcpu_index: ffi::c_uint, info: qp::QemuPluginMeminfo, vaddr: u64, userdata: *mut ffi::c_void) {
    let conf = get_conf();
    
    let u: &UserdataMem = unsafe { &*(userdata as *const UserdataMem) };

    // handle checkpoint up here because it doesn't count as a normal memory access
    // otherwise we would skip it as it is not in pmem range
    if let UserdataMem::Checkpoint = u {
        let mut value: u8 = 0;
        unsafe { qp::qemu_plugin_vcpu_memory_rw(vcpu_index, vaddr, &mut value as *mut _ as *mut ffi::c_void, 1, false, false) };
        if value == 255 { // special value when kernel is booted
            initialize_pmem_area();
        }

        if conf.trace_what.contains(TraceOption::Checkpoint) {
            send_msg(TraceMessage::Checkpoint { value });
        }
        return;
    }

    let paddr = unsafe { qp::qemu_plugin_hwaddr_phys_addr(qp::qemu_plugin_get_hwaddr(info, vaddr)) };
    if paddr < conf.pmem_start || paddr >= conf.pmem_start + conf.pmem_len {
        return;
    }
    let address = paddr - conf.pmem_start;
    // *HAVE_WRITES.lock().unwrap() = true;

    match u {
        // filtering of checkpoint and flush happens in hook_insn
        UserdataMem::Checkpoint => panic!("checkpoints handled above"),
        UserdataMem::Clflush { .. } => {
            // *HAVE_WRITES.lock().unwrap() = true;
            send_msg(TraceMessage::Pmem(PmemEvent::Clflush { address }));
        },
        UserdataMem::Clflushopt { .. } => {
            // *HAVE_WRITES.lock().unwrap() = true;
            send_msg(TraceMessage::Pmem(PmemEvent::Clflushopt { address }));
        },
        UserdataMem::Clwb { .. } => {
            // *HAVE_WRITES.lock().unwrap() = true;
            send_msg(TraceMessage::Pmem(PmemEvent::Clwb { address }));
        },
        UserdataMem::ReadWrite { disas: _, nt: is_nt } => {
            let is_store = unsafe { qp::qemu_plugin_mem_is_store(info) };
            if (is_store && conf.trace_what.contains(TraceOption::PmemWrite))
                    || (!is_store && conf.trace_what.contains(TraceOption::PmemRead)) {
                let nb = unsafe { 1usize << qp::qemu_plugin_mem_size_shift(info) };
                let mut buf: Vec<u8> = Vec::with_capacity(nb);
                unsafe {
                    // TODO we could now do this with paddr as well.
                    qp::qemu_plugin_vcpu_memory_rw(vcpu_index, vaddr, buf.as_mut_ptr() as *mut ffi::c_void, nb as u64, false, false);
                    // safety: we assume that nb elements have been read (and are now initialized)
                    buf.set_len(nb);
                }

                if is_store {
                    send_msg(TraceMessage::Pmem(PmemEvent::Write { address, size: nb as u64, content: buf, non_temporal: *is_nt }));
                } else {
                    send_msg(TraceMessage::Pmem(PmemEvent::Read { address, size: nb as u64, content: buf }));
                }
            }
        },
    }
}

// TODO instead of having one exec_cb and one mem_cb, we could have several ones, and avoid
// branching inside the callbacks

fn hook_insn(insn: *mut qp::QemuPluginInsn) {
    let conf = get_conf();

    let data = unsafe {
        let dataptr = qp::qemu_plugin_insn_data(insn) as *const u8;
        let datasize = qp::qemu_plugin_insn_size(insn);
        if dataptr.is_null() || datasize == 0 {
            panic!("qemu_plugin_insn_(data|size) invalid return value");
        }
        std::slice::from_raw_parts(dataptr, datasize)
    };
    let this_is_checkpoint = *HAVE_CHECKPOINT_SIGNAL_INSN.lock().unwrap();
    let checkpoint_signal_insn: [u8; 5] = [0xb8, 0x70, 0x65, 0x72, 0x6d];
    // next instruction coming up is going to be a memory write with the value of the checkpoint
    // we set this globally at the end of the function so that the next invocation sees it
    *HAVE_CHECKPOINT_SIGNAL_INSN.lock().unwrap() = data == &checkpoint_signal_insn[..];

    let mut decoder = Decoder::new(64, data, DecoderOptions::NONE);
    let decoded_insn = decoder.decode();
    if decoded_insn.is_invalid() {
        if cfg!(permanent_trace_insn_invalid = "panic") {
            panic!("invalid instruction: {:02x?}", data);
        } else if cfg!(permanent_trace_insn_invalid = "print") {
            println!("permanent_plugin: invalid instruction: {:02x?}", data);
            // NOTE: this leaks memory
            let qemu_disas = unsafe { ffi::CStr::from_ptr(qp::qemu_plugin_insn_disas(insn)).to_str().unwrap() };
            println!("permanent_plugin:  {}", qemu_disas);
        }
        return;
    }
    let disas = decoded_insn.to_string();
    
    let maybe_exec_udat = match decoded_insn.mnemonic() {
        Mnemonic::Wbinvd => conf.trace_what.contains(TraceOption::PmemFlush)
                .then(|| Box::new(UserdataExec::Wbinvd { disas: disas.clone() })),
        Mnemonic::Mfence | Mnemonic::Sfence => conf.trace_what.contains(TraceOption::PmemFence)
                .then(|| Box::new(UserdataExec::Fence { disas: disas.clone() })),
        _ => None
    };

    if let Some(mut exec_udat) = maybe_exec_udat {
        unsafe {
            qp::qemu_plugin_register_vcpu_insn_exec_cb(insn,
                Some(my_vcpu_insn_exec_cb),
                qp::QEMU_PLUGIN_CB_R_REGS,
                &mut *exec_udat as *mut _ as *mut ffi::c_void);
        }
        USERDATA_EXEC_VEC.lock().unwrap().push(exec_udat);
    }
    
    let trace_rw = conf.trace_what.contains(TraceOption::PmemRead)
        || conf.trace_what.contains(TraceOption::PmemWrite);
    // we do some filtering of mem events inside the callback, because we don't know at this point (and QEMU is weird)
    let maybe_mem_udat = match decoded_insn.mnemonic() {
        _ if this_is_checkpoint => Some(Box::new(UserdataMem::Checkpoint)), // always checkpoint,
                                                                            // because we use it
                                                                            // for pmem init
        Mnemonic::Clflush => conf.trace_what.contains(TraceOption::PmemFlush)
                .then(|| Box::new(UserdataMem::Clflush { disas })),
        Mnemonic::Clflushopt => conf.trace_what.contains(TraceOption::PmemFlush)
                .then(|| Box::new(UserdataMem::Clflushopt { disas })),
        Mnemonic::Clwb => conf.trace_what.contains(TraceOption::PmemFlush)
                .then(|| Box::new(UserdataMem::Clwb { disas })),
        Mnemonic::Movntdq
        | Mnemonic::Movntdqa
        | Mnemonic::Movnti
        | Mnemonic::Movntpd
        | Mnemonic::Movntps
        | Mnemonic::Movntq
        | Mnemonic::Movntsd
        | Mnemonic::Movntss
            => trace_rw.then(|| Box::new(UserdataMem::ReadWrite { disas, nt: true })),
        _ => trace_rw.then(|| Box::new(UserdataMem::ReadWrite { disas, nt: false })),
    };

    if let Some(mut mem_udat) = maybe_mem_udat {
        unsafe {
            qp::qemu_plugin_register_vcpu_mem_cb(insn,
                Some(my_vcpu_mem_cb),
                qp::QEMU_PLUGIN_CB_R_REGS,
                qp::QEMU_PLUGIN_MEM_RW,
                &mut *mem_udat as *mut _ as *mut ffi::c_void);
        }
        USERDATA_MEM_VEC.lock().unwrap().push(mem_udat);
    }
}

fn initialize_pmem_area() {
    let mut have_pmem_init = HAVE_PMEM_INIT.lock().unwrap();
    if *have_pmem_init {
        panic!("pmem initialized twice");
    }
    *have_pmem_init = true;

    let conf = get_conf();
    if conf.pmem_len == 0 { // no pmem
        return;
    }
    let mut data = match &conf.pmem_base_image_path {
        Some(path) => {
            println!("permanent_plugin: initialize pmem from file {}", path);
            let content = std::fs::read(path).unwrap();
            if content.len() != conf.pmem_len as usize {
                panic!("pmem_base_image file has the wrong size");
            }
            content
        },
        None => {
            println!("permanent_plugin: initialize pmem as zero");
            vec![0u8; conf.pmem_len as usize]
        }
    };
    unsafe { qp::qemu_plugin_vcpu_memory_rw(0, conf.pmem_start, data.as_mut_ptr() as *mut ffi::c_void, conf.pmem_len, true, true) };
    println!("permanent_plugin: pmem initialized");
}

#[no_mangle]
extern "C" fn my_vcpu_tb_trans_cb(_id: qp::QemuPluginId, tb: *mut qp::QemuPluginTb) {
    let n = unsafe { qp::qemu_plugin_tb_n_insns(tb) };
    for i in 0..n {
        let insn = unsafe { qp::qemu_plugin_tb_get_insn(tb, i) };
        hook_insn(insn);
    }
}

#[no_mangle]
extern "C" fn my_atexit_cb(_id: qp::QemuPluginId, _userdata: *mut ffi::c_void) {
    // now we can drop the userdata
    USERDATA_MEM_VEC.lock().unwrap().clear();
    USERDATA_EXEC_VEC.lock().unwrap().clear();
    
    // collect writer thread
    let wt = mem::take(&mut *WRITER_THREAD.lock().unwrap()).unwrap();
    wt.done_send.send(()).expect("couldn't signal writer thread termination");
    wt.handle.join().expect("couldn't join writer thread");
}

#[no_mangle]
pub extern "C" fn qemu_plugin_install(
        id: qp::QemuPluginId,
        _info: *const qp::QemuInfo,
        argc: ffi::c_int,
        argv: *mut *mut ffi::c_char,
    ) -> ffi::c_int
{
    println!("permanent_plugin: install");
    // create config
    let argc: usize = argc.try_into().unwrap();
    let argv_slice = unsafe { std::slice::from_raw_parts(argv, argc) };
    let mut args = Vec::new();
    for i in 0..argc {
        let arg = unsafe { ffi::CStr::from_ptr(argv_slice[i]).to_str().unwrap() };
        args.push(arg);
    }
    let mut conf = TcgPluginConfig {
        pmem_start: 0,
        pmem_len: 0,
        pmem_base_image_path: None,
        trace_what: EnumSet::empty(),
        out_trace_file: String::new(),
    };
    for arg in args {
        let (key, value) = arg.split_once("=").expect("invalid argument");
        // TODO return 1 instead of unwrap
        match key {
            "pmem_start" => { conf.pmem_start = value.parse().unwrap(); }, // TODO might overflow if we set pmem_start but not pmem_len
            "pmem_len" => { conf.pmem_len = value.parse().unwrap(); },
            "trace_what" => {
                let trace_what_args: Vec<&str> = value.split("/").collect();
                for arg in trace_what_args {
                    match TraceOption::from_qemu_str(arg) {
                        Ok(opt) => { conf.trace_what.insert(opt); }
                        Err(()) => panic!("unknown trace_what argument: {}", arg),
                    }
                }
            },
            "pmem_base_image_path" => { conf.pmem_base_image_path = Some(value.to_string()); },
            "out_trace_file" => {
                conf.out_trace_file = value.to_string();
            },
            _ => panic!("unknown argument: {}", key),
        }
    }

    unsafe {
        // NOTE: removed performance optimization for post-failure tracing because we
        // initialize pmem area inside the mem callback
        qp::qemu_plugin_register_vcpu_tb_trans_cb(id, Some(my_vcpu_tb_trans_cb));
        qp::qemu_plugin_register_atexit_cb(id, Some(my_atexit_cb), std::ptr::null_mut::<ffi::c_void>());
    }

    let trace_out = new_trace_writer_bin(File::create(&conf.out_trace_file).expect("could not open out_trace_file"));
    CONFIG.set(conf).expect("could not set config");

    // create writer thread
    let (trace_send, trace_recv) = crossbeam_channel::unbounded::<TraceMessage>();
    let (done_send, done_recv) = crossbeam_channel::bounded(0);
    println!("permanent_plugin: start writer thread");
    let handle = std::thread::spawn(move || { writer_main(trace_recv, done_recv, trace_out); });
    *WRITER_THREAD.lock().unwrap() = Some(WriterThread { trace_send, done_send, handle });

    println!("permanent_plugin: install successful");
    0
}

//------------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn permanent_trace_pci_nvme_read(_c_cid: u16, _nsid: u32, _nlb: u32, _count: u64, _lba: u64) {
    /* ignore */
}

#[no_mangle]
pub extern "C" fn permanent_trace_pci_nvme_write(_c_cid: u16, _verb: *const ffi::c_char, _nsid: u32, _nlb: u32, _count: u64, _lba: u64) {
    /* ignore */
}

#[no_mangle]
pub extern "C" fn permanent_trace_pci_nvme_io_cmd(_cid: u16, _nsid: u32, _sqid: u16, _opcode: u8, opname: *const ffi::c_char) {
    if get_conf().trace_what.contains(TraceOption::NvmeFlush) {
        unsafe {
            match ffi::CStr::from_ptr(opname).to_str().unwrap() {
                "NVME_NVM_CMD_FLUSH" => {
                    send_msg(TraceMessage::NvmeFlush);
                },
                _ => { /* ignore */ },
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn permanent_trace_pci_nvme_blk_read(req: *const ffi::c_void, offset: u64) {
    if get_conf().trace_what.contains(TraceOption::NvmeRead) {
        send_msg(TraceMessage::PciNvmeBlkRead { req: req as u64, offset });
    }
}

#[no_mangle]
pub extern "C" fn permanent_trace_pci_nvme_blk_write(req: *const ffi::c_void, offset: u64) {
    if get_conf().trace_what.contains(TraceOption::NvmeWrite) {
        send_msg(TraceMessage::PciNvmeBlkWrite { req: req as u64, offset });
    }
}

#[no_mangle]
pub extern "C" fn permanent_trace_pci_nvme_enqueue_req_completion(req: *const ffi::c_void, status: u16) {
    if (get_conf().trace_what.contains(TraceOption::NvmeRead)
            || get_conf().trace_what.contains(TraceOption::NvmeWrite))
            && status == 0 {
        send_msg(TraceMessage::PciNvmeEnqueueReqCompletion { req: req as u64 });
    }
}

#[no_mangle]
pub extern "C" fn permanent_trace_dma_blk_read(dbs: *const ffi::c_void, offset: i64, bytes: i64) {
    if get_conf().trace_what.contains(TraceOption::NvmeRead) {
        send_msg(TraceMessage::DmaBlkRead { dbs: dbs as u64, offset, length: bytes });
    }
}

#[no_mangle]
pub extern "C" fn permanent_trace_dma_blk_write(dbs: *const ffi::c_void, offset: i64, bytes: i64, buf: *const ffi::c_void) {
    if get_conf().trace_what.contains(TraceOption::NvmeWrite) {
        let nb: usize = bytes.try_into().unwrap();
        let mut data: Vec<u8> = Vec::with_capacity(nb);
        unsafe {
            std::ptr::copy_nonoverlapping(buf as *const u8, data.as_mut_ptr(), nb);
            data.set_len(nb);
        }
        send_msg(TraceMessage::DmaBlkWrite { dbs: dbs as u64, offset, length: bytes, data });
    }
}

#[no_mangle]
pub extern "C" fn permanent_trace_dma_blk_io(req: *const ffi::c_void, dbs: *const ffi::c_void) {
    if get_conf().trace_what.contains(TraceOption::NvmeRead)
            || get_conf().trace_what.contains(TraceOption::NvmeWrite) {
        send_msg(TraceMessage::DmaBlkIo { req: req as u64, dbs: dbs as u64 });
    }
}
