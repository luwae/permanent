use std::io::{self, BufWriter};
use std::fs::File;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::process::{Child, Command, Stdio};
use enumset::EnumSet;
use crate::pipe::Pipe;
extern crate libc;

use permanent_common::config::{VmConfig, TraceConfig, TraceType, TraceOption, TcgPluginConfig};

pub struct VM {
    pipe: Pipe,
    process: Child,
}

impl VM {
    // NOTE: we don't need TestConfig here, because we only start the VM (independent of test conf)
    pub fn init(vm_config: &VmConfig, trace_config: &TraceConfig) -> Self {
        println!("Create VM");

        let io_log_file = File::create(&trace_config.io_log_path()).expect("Could not create io log file");
        let log_file = File::create(&trace_config.log_path()).expect("Could not create log file");

        let mut command = Command::new(&vm_config.qemu_path);
        // add kernel, initrd
        command.args(["-kernel", vm_config.kernel_path.as_str()]);
        command.args(["-initrd", vm_config.initrd_path.as_str()]);

        // pipe interface
        command.args(["-serial", format!("pipe:{}", &trace_config.pipe_path()).as_str()]);
        command.arg("-nographic");
        
        // add nvme drive, if required
        let (_, uses_nvme) = vm_config.have_pmem_nvme();
        if uses_nvme {
            command.args([
                "-drive", format!("file={},format=raw,if=none,id=nvm", trace_config.nvme_image_path()).as_str(),
                "-device", "nvme,serial=deadbeef,drive=nvm",
            ]);
        }

        let (p, n) = vm_config.have_pmem_nvme();

        // add plugin information
        let pmem_trace_what = TraceOption::PmemWrite | TraceOption::PmemFence | TraceOption::PmemFlush;
        let nvme_trace_what = TraceOption::NvmeWrite | TraceOption::NvmeFlush;
        let plugin_config = TcgPluginConfig {
            pmem_start: vm_config.pmem_start.unwrap_or(0),
            pmem_len: vm_config.pmem_len.unwrap_or(0),
            pmem_base_image_path: p.then(|| trace_config.pmem_image_path()),
            trace_what: match trace_config.trace_type {
                TraceType::Analyse => {
                    let mut opts = TraceOption::Checkpoint.into();
                    if p { opts |= pmem_trace_what; }
                    if n { opts |= nvme_trace_what; }
                    opts
                },
                TraceType::PostSuccess => {
                    let mut opts = TraceOption::Checkpoint.into();
                    if p { opts |= TraceOption::PmemRead; }
                    if n { opts |= TraceOption::NvmeRead; }
                    opts
                },
                TraceType::PostFailure { .. } => EnumSet::empty(),
            },
            out_trace_file: trace_config.trace_path(),
        };
        command.args([
            "-plugin",
            plugin_config.to_qemu_plugin_arg_string("target/release/libpermanent_plugin.so").as_str()
        ]);

        // add free-form qemu args
        command.args(vm_config.qemu_args.clone());

        command.stderr(unsafe { Stdio::from_raw_fd(io_log_file.into_raw_fd()) });

        println!("== Start QEMU VM");
        println!("{:?}", command);

        let child = command.spawn().expect("Could not start qemu vm");

        // TODO lower?
        std::thread::sleep(std::time::Duration::from_millis(2000));
        let mut pipe = Pipe::open(&trace_config.pipe_path(), BufWriter::new(log_file)).expect("Could not open control pipe");
        println!("Pipes opened");

        pipe.wait_for(b"/bin/sh: can't access tty; job control turned off").unwrap();
        println!("VM ready");

        return Self { pipe, process: child };
    }

    pub fn teardown(&mut self) -> bool {
        let variants = [b"PERMANENT SUCCESS".as_slice(), b"PERMANENT FAIL".as_slice()];
        let success = self.pipe.wait_for_any(&variants).unwrap() == 0; // make sure we collected all output
        // send SIGTERM for qemu to terminate gracefully
        unsafe { libc::kill(self.process.id() as i32, libc::SIGTERM); }

        self.process.wait().expect("Could not collect qemu");
        println!("== Exit QEMU VM");
        return success;
    }

    pub fn send(&mut self, text: &str) -> Result<(), io::Error> {
        self.pipe.send(text)
    }
}
