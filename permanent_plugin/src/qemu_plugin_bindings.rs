#![allow(dead_code)]

pub const QEMU_PLUGIN_VERSION: u32 = 2;
#[doc = " typedef QemuPluginId - Unique plugin ID"]
pub type QemuPluginId = u64;
extern "C" {
    pub static mut qemu_plugin_version: ::core::ffi::c_int;
}
#[doc = " struct QemuInfo - system information for plugins\n\n This structure provides for some limited information about the\n system to allow the plugin to make decisions on how to proceed. For\n example it might only be suitable for running on some guest\n architectures or when under full system emulation."]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct QemuInfo {
    #[doc = " @target_name: string describing architecture"]
    pub target_name: *const ::core::ffi::c_char,
    pub version: QemuInfoBindgenTy1,
    #[doc = " @system_emulation: is this a full system emulation?"]
    pub system_emulation: bool,
    pub __bindgen_anon_1: QemuInfoBindgenTy2,
}
#[doc = " @version: minimum and current plugin API level"]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct QemuInfoBindgenTy1 {
    pub min: ::core::ffi::c_int,
    pub cur: ::core::ffi::c_int,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union QemuInfoBindgenTy2 {
    pub system: QemuInfoBindgenTy2BindgenTy1,
}
#[doc = " @system: information relevant to system emulation"]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct QemuInfoBindgenTy2BindgenTy1 {
    #[doc = " @system.smp_vcpus: initial number of vCPUs"]
    pub smp_vcpus: ::core::ffi::c_int,
    #[doc = " @system.max_vcpus: maximum possible number of vCPUs"]
    pub max_vcpus: ::core::ffi::c_int,
}
extern "C" {
    #[doc = " qemu_plugin_install() - Install a plugin\n @id: this plugin's opaque ID\n @info: a block describing some details about the guest\n @argc: number of arguments\n @argv: array of arguments (@argc elements)\n\n All plugins must export this symbol which is called when the plugin\n is first loaded. Calling qemu_plugin_uninstall() from this function\n is a bug.\n\n Note: @info is only live during the call. Copy any information we\n want to keep. @argv remains valid throughout the lifetime of the\n loaded plugin.\n\n Return: 0 on successful loading, !0 for an error."]
    pub fn qemu_plugin_install(
        id: QemuPluginId,
        info: *const QemuInfo,
        argc: ::core::ffi::c_int,
        argv: *mut *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
#[doc = " typedef QemuPluginSimpleCb - simple callback\n @id: the unique QemuPluginId\n\n This callback passes no information aside from the unique @id."]
pub type QemuPluginSimpleCb =
    ::core::option::Option<unsafe extern "C" fn(id: QemuPluginId)>;
#[doc = " typedef QemuPluginUdataCb - callback with user data\n @id: the unique QemuPluginId\n @userdata: a pointer to some user data supplied when the callback\n was registered."]
pub type QemuPluginUdataCb = ::core::option::Option<
    unsafe extern "C" fn(id: QemuPluginId, userdata: *mut ::core::ffi::c_void),
>;
#[doc = " typedef QemuPluginVcpuSimpleCb - vcpu callback\n @id: the unique QemuPluginId\n @vcpu_index: the current vcpu context"]
pub type QemuPluginVcpuSimpleCb = ::core::option::Option<
    unsafe extern "C" fn(id: QemuPluginId, vcpu_index: ::core::ffi::c_uint),
>;
#[doc = " typedef QemuPluginVcpuUdataCb - vcpu callback\n @vcpu_index: the current vcpu context\n @userdata: a pointer to some user data supplied when the callback\n was registered."]
pub type QemuPluginVcpuUdataCb = ::core::option::Option<
    unsafe extern "C" fn(vcpu_index: ::core::ffi::c_uint, userdata: *mut ::core::ffi::c_void),
>;
extern "C" {
    #[doc = " qemu_plugin_uninstall() - Uninstall a plugin\n @id: this plugin's opaque ID\n @cb: callback to be called once the plugin has been removed\n\n Do NOT assume that the plugin has been uninstalled once this function\n returns. Plugins are uninstalled asynchronously, and therefore the given\n plugin receives callbacks until @cb is called.\n\n Note: Calling this function from qemu_plugin_install() is a bug."]
    pub fn qemu_plugin_uninstall(id: QemuPluginId, cb: QemuPluginSimpleCb);
}
extern "C" {
    #[doc = " qemu_plugin_reset() - Reset a plugin\n @id: this plugin's opaque ID\n @cb: callback to be called once the plugin has been reset\n\n Unregisters all callbacks for the plugin given by @id.\n\n Do NOT assume that the plugin has been reset once this function returns.\n Plugins are reset asynchronously, and therefore the given plugin receives\n callbacks until @cb is called."]
    pub fn qemu_plugin_reset(id: QemuPluginId, cb: QemuPluginSimpleCb);
}
extern "C" {
    #[doc = " qemu_plugin_register_vcpu_init_cb() - register a vCPU initialization callback\n @id: plugin ID\n @cb: callback function\n\n The @cb function is called every time a vCPU is initialized.\n\n See also: qemu_plugin_register_vcpu_exit_cb()"]
    pub fn qemu_plugin_register_vcpu_init_cb(
        id: QemuPluginId,
        cb: QemuPluginVcpuSimpleCb,
    );
}
extern "C" {
    #[doc = " qemu_plugin_register_vcpu_exit_cb() - register a vCPU exit callback\n @id: plugin ID\n @cb: callback function\n\n The @cb function is called every time a vCPU exits.\n\n See also: qemu_plugin_register_vcpu_init_cb()"]
    pub fn qemu_plugin_register_vcpu_exit_cb(
        id: QemuPluginId,
        cb: QemuPluginVcpuSimpleCb,
    );
}
extern "C" {
    #[doc = " qemu_plugin_register_vcpu_idle_cb() - register a vCPU idle callback\n @id: plugin ID\n @cb: callback function\n\n The @cb function is called every time a vCPU idles."]
    pub fn qemu_plugin_register_vcpu_idle_cb(
        id: QemuPluginId,
        cb: QemuPluginVcpuSimpleCb,
    );
}
extern "C" {
    #[doc = " qemu_plugin_register_vcpu_resume_cb() - register a vCPU resume callback\n @id: plugin ID\n @cb: callback function\n\n The @cb function is called every time a vCPU resumes execution."]
    pub fn qemu_plugin_register_vcpu_resume_cb(
        id: QemuPluginId,
        cb: QemuPluginVcpuSimpleCb,
    );
}
#[doc = " struct QemuPluginTb - Opaque handle for a translation block"]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct QemuPluginTb {
    _unused: [u8; 0],
}
#[doc = " struct QemuPluginInsn - Opaque handle for a translated instruction"]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct QemuPluginInsn {
    _unused: [u8; 0],
}
pub const QEMU_PLUGIN_CB_NO_REGS: QemuPluginCbFlags = 0;
pub const QEMU_PLUGIN_CB_R_REGS: QemuPluginCbFlags = 1;
pub const QEMU_PLUGIN_CB_RW_REGS: QemuPluginCbFlags = 2;
#[doc = " enum QemuPluginCbFlags - type of callback\n\n @QEMU_PLUGIN_CB_NO_REGS: callback does not access the CPU's regs\n @QEMU_PLUGIN_CB_R_REGS: callback reads the CPU's regs\n @QEMU_PLUGIN_CB_RW_REGS: callback reads and writes the CPU's regs\n\n Note: currently unused, plugins cannot read or change system\n register state."]
pub type QemuPluginCbFlags = ::core::ffi::c_uint;
pub const QEMU_PLUGIN_MEM_R: QemuPluginMemRw = 1;
pub const QEMU_PLUGIN_MEM_W: QemuPluginMemRw = 2;
pub const QEMU_PLUGIN_MEM_RW: QemuPluginMemRw = 3;
pub type QemuPluginMemRw = ::core::ffi::c_uint;
#[doc = " typedef QemuPluginVcpuTbTransCb - translation callback\n @id: unique plugin id\n @tb: opaque handle used for querying and instrumenting a block."]
pub type QemuPluginVcpuTbTransCb =
    ::core::option::Option<unsafe extern "C" fn(id: QemuPluginId, tb: *mut QemuPluginTb)>;
extern "C" {
    #[doc = " qemu_plugin_register_vcpu_tb_trans_cb() - register a translate cb\n @id: plugin ID\n @cb: callback function\n\n The @cb function is called every time a translation occurs. The @cb\n function is passed an opaque qemu_plugin_type which it can query\n for additional information including the list of translated\n instructions. At this point the plugin can register further\n callbacks to be triggered when the block or individual instruction\n executes."]
    pub fn qemu_plugin_register_vcpu_tb_trans_cb(
        id: QemuPluginId,
        cb: QemuPluginVcpuTbTransCb,
    );
}
extern "C" {
    #[doc = " qemu_plugin_register_vcpu_tb_exec_cb() - register execution callback\n @tb: the opaque QemuPluginTb handle for the translation\n @cb: callback function\n @flags: does the plugin read or write the CPU's registers?\n @userdata: any plugin data to pass to the @cb?\n\n The @cb function is called every time a translated unit executes."]
    pub fn qemu_plugin_register_vcpu_tb_exec_cb(
        tb: *mut QemuPluginTb,
        cb: QemuPluginVcpuUdataCb,
        flags: QemuPluginCbFlags,
        userdata: *mut ::core::ffi::c_void,
    );
}
pub const QEMU_PLUGIN_INLINE_ADD_U64: QemuPluginOp = 0;
#[doc = " enum QemuPluginOp - describes an inline op\n\n @QEMU_PLUGIN_INLINE_ADD_U64: add an immediate value uint64_t\n\n Note: currently only a single inline op is supported."]
pub type QemuPluginOp = ::core::ffi::c_uint;
extern "C" {
    #[doc = " qemu_plugin_register_vcpu_tb_exec_inline() - execution inline op\n @tb: the opaque QemuPluginTb handle for the translation\n @op: the type of QemuPluginOp (e.g. ADD_U64)\n @ptr: the target memory location for the op\n @imm: the op data (e.g. 1)\n\n Insert an inline op to every time a translated unit executes.\n Useful if you just want to increment a single counter somewhere in\n memory.\n\n Note: ops are not atomic so in multi-threaded/multi-smp situations\n you will get inexact results."]
    pub fn qemu_plugin_register_vcpu_tb_exec_inline(
        tb: *mut QemuPluginTb,
        op: QemuPluginOp,
        ptr: *mut ::core::ffi::c_void,
        imm: u64,
    );
}
extern "C" {
    #[doc = " qemu_plugin_register_vcpu_insn_exec_cb() - register insn execution cb\n @insn: the opaque QemuPluginInsn handle for an instruction\n @cb: callback function\n @flags: does the plugin read or write the CPU's registers?\n @userdata: any plugin data to pass to the @cb?\n\n The @cb function is called every time an instruction is executed"]
    pub fn qemu_plugin_register_vcpu_insn_exec_cb(
        insn: *mut QemuPluginInsn,
        cb: QemuPluginVcpuUdataCb,
        flags: QemuPluginCbFlags,
        userdata: *mut ::core::ffi::c_void,
    );
}
extern "C" {
    #[doc = " qemu_plugin_register_vcpu_insn_exec_inline() - insn execution inline op\n @insn: the opaque QemuPluginInsn handle for an instruction\n @op: the type of QemuPluginOp (e.g. ADD_U64)\n @ptr: the target memory location for the op\n @imm: the op data (e.g. 1)\n\n Insert an inline op to every time an instruction executes. Useful\n if you just want to increment a single counter somewhere in memory."]
    pub fn qemu_plugin_register_vcpu_insn_exec_inline(
        insn: *mut QemuPluginInsn,
        op: QemuPluginOp,
        ptr: *mut ::core::ffi::c_void,
        imm: u64,
    );
}
extern "C" {
    #[doc = " qemu_plugin_tb_n_insns() - query helper for number of insns in TB\n @tb: opaque handle to TB passed to callback\n\n Returns: number of instructions in this block"]
    pub fn qemu_plugin_tb_n_insns(tb: *const QemuPluginTb) -> usize;
}
extern "C" {
    #[doc = " qemu_plugin_tb_vaddr() - query helper for vaddr of TB start\n @tb: opaque handle to TB passed to callback\n\n Returns: virtual address of block start"]
    pub fn qemu_plugin_tb_vaddr(tb: *const QemuPluginTb) -> u64;
}
extern "C" {
    #[doc = " qemu_plugin_tb_get_insn() - retrieve handle for instruction\n @tb: opaque handle to TB passed to callback\n @idx: instruction number, 0 indexed\n\n The returned handle can be used in follow up helper queries as well\n as when instrumenting an instruction. It is only valid for the\n lifetime of the callback.\n\n Returns: opaque handle to instruction"]
    pub fn qemu_plugin_tb_get_insn(tb: *const QemuPluginTb, idx: usize) -> *mut QemuPluginInsn;
}
extern "C" {
    #[doc = " qemu_plugin_insn_data() - return ptr to instruction data\n @insn: opaque instruction handle from qemu_plugin_tb_get_insn()\n\n Note: data is only valid for duration of callback. See\n qemu_plugin_insn_size() to calculate size of stream.\n\n Returns: pointer to a stream of bytes containing the value of this\n instructions opcode."]
    pub fn qemu_plugin_insn_data(insn: *const QemuPluginInsn) -> *const ::core::ffi::c_void;
}
extern "C" {
    #[doc = " qemu_plugin_insn_size() - return size of instruction\n @insn: opaque instruction handle from qemu_plugin_tb_get_insn()\n\n Returns: size of instruction in bytes"]
    pub fn qemu_plugin_insn_size(insn: *const QemuPluginInsn) -> usize;
}
extern "C" {
    #[doc = " qemu_plugin_insn_vaddr() - return vaddr of instruction\n @insn: opaque instruction handle from qemu_plugin_tb_get_insn()\n\n Returns: virtual address of instruction"]
    pub fn qemu_plugin_insn_vaddr(insn: *const QemuPluginInsn) -> u64;
}
extern "C" {
    #[doc = " qemu_plugin_insn_haddr() - return hardware addr of instruction\n @insn: opaque instruction handle from qemu_plugin_tb_get_insn()\n\n Returns: hardware (physical) target address of instruction"]
    pub fn qemu_plugin_insn_haddr(insn: *const QemuPluginInsn) -> *mut ::core::ffi::c_void;
}
#[doc = " typedef QemuPluginMeminfo - opaque memory transaction handle\n\n This can be further queried using the qemu_plugin_mem_* query\n functions."]
pub type QemuPluginMeminfo = u32;
#[doc = " struct QemuPluginHwaddr - opaque hw address handle"]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct QemuPluginHwaddr {
    _unused: [u8; 0],
}
extern "C" {
    #[doc = " qemu_plugin_mem_size_shift() - get size of access\n @info: opaque memory transaction handle\n\n Returns: size of access in ^2 (0=byte, 1=16bit, 2=32bit etc...)"]
    pub fn qemu_plugin_mem_size_shift(info: QemuPluginMeminfo) -> ::core::ffi::c_uint;
}
extern "C" {
    #[doc = " qemu_plugin_mem_is_sign_extended() - was the access sign extended\n @info: opaque memory transaction handle\n\n Returns: true if it was, otherwise false"]
    pub fn qemu_plugin_mem_is_sign_extended(info: QemuPluginMeminfo) -> bool;
}
extern "C" {
    #[doc = " qemu_plugin_mem_is_big_endian() - was the access big endian\n @info: opaque memory transaction handle\n\n Returns: true if it was, otherwise false"]
    pub fn qemu_plugin_mem_is_big_endian(info: QemuPluginMeminfo) -> bool;
}
extern "C" {
    #[doc = " qemu_plugin_mem_is_store() - was the access a store\n @info: opaque memory transaction handle\n\n Returns: true if it was, otherwise false"]
    pub fn qemu_plugin_mem_is_store(info: QemuPluginMeminfo) -> bool;
}
extern "C" {
    #[doc = " qemu_plugin_get_hwaddr() - return handle for memory operation\n @info: opaque memory info structure\n @vaddr: the virtual address of the memory operation\n\n For system emulation returns a QemuPluginHwaddr handle to query\n details about the actual physical address backing the virtual\n address. For linux-user guests it just returns NULL.\n\n This handle is *only* valid for the duration of the callback. Any\n information about the handle should be recovered before the\n callback returns."]
    pub fn qemu_plugin_get_hwaddr(
        info: QemuPluginMeminfo,
        vaddr: u64,
    ) -> *mut QemuPluginHwaddr;
}
extern "C" {
    #[doc = " qemu_plugin_hwaddr_is_io() - query whether memory operation is IO\n @haddr: address handle from qemu_plugin_get_hwaddr()\n\n Returns true if the handle's memory operation is to memory-mapped IO, or\n false if it is to RAM"]
    pub fn qemu_plugin_hwaddr_is_io(haddr: *const QemuPluginHwaddr) -> bool;
}
extern "C" {
    #[doc = " qemu_plugin_hwaddr_phys_addr() - query physical address for memory operation\n @haddr: address handle from qemu_plugin_get_hwaddr()\n\n Returns the physical address associated with the memory operation\n\n Note that the returned physical address may not be unique if you are dealing\n with multiple address spaces."]
    pub fn qemu_plugin_hwaddr_phys_addr(haddr: *const QemuPluginHwaddr) -> u64;
}
extern "C" {
    pub fn qemu_plugin_hwaddr_device_name(
        h: *const QemuPluginHwaddr,
    ) -> *const ::core::ffi::c_char;
}
#[doc = " typedef QemuPluginVcpuMemCb - memory callback function type\n @vcpu_index: the executing vCPU\n @info: an opaque handle for further queries about the memory\n @vaddr: the virtual address of the transaction\n @userdata: any user data attached to the callback"]
pub type QemuPluginVcpuMemCb = ::core::option::Option<
    unsafe extern "C" fn(
        vcpu_index: ::core::ffi::c_uint,
        info: QemuPluginMeminfo,
        vaddr: u64,
        userdata: *mut ::core::ffi::c_void,
    ),
>;
extern "C" {
    #[doc = " qemu_plugin_register_vcpu_mem_cb() - register memory access callback\n @insn: handle for instruction to instrument\n @cb: callback of type QemuPluginVcpuMemCb\n @flags: (currently unused) callback flags\n @rw: monitor reads, writes or both\n @userdata: opaque pointer for userdata\n\n This registers a full callback for every memory access generated by\n an instruction. If the instruction doesn't access memory no\n callback will be made.\n\n The callback reports the vCPU the access took place on, the virtual\n address of the access and a handle for further queries. The user\n can attach some userdata to the callback for additional purposes.\n\n Other execution threads will continue to execute during the\n callback so the plugin is responsible for ensuring it doesn't get\n confused by making appropriate use of locking if required."]
    pub fn qemu_plugin_register_vcpu_mem_cb(
        insn: *mut QemuPluginInsn,
        cb: QemuPluginVcpuMemCb,
        flags: QemuPluginCbFlags,
        rw: QemuPluginMemRw,
        userdata: *mut ::core::ffi::c_void,
    );
}
extern "C" {
    #[doc = " qemu_plugin_register_vcpu_mem_inline() - register an inline op to any memory access\n @insn: handle for instruction to instrument\n @rw: apply to reads, writes or both\n @op: the op, of type QemuPluginOp\n @ptr: pointer memory for the op\n @imm: immediate data for @op\n\n This registers a inline op every memory access generated by the\n instruction. This provides for a lightweight but not thread-safe\n way of counting the number of operations done."]
    pub fn qemu_plugin_register_vcpu_mem_inline(
        insn: *mut QemuPluginInsn,
        rw: QemuPluginMemRw,
        op: QemuPluginOp,
        ptr: *mut ::core::ffi::c_void,
        imm: u64,
    );
}
pub type QemuPluginVcpuSyscallCb = ::core::option::Option<
    unsafe extern "C" fn(
        id: QemuPluginId,
        vcpu_index: ::core::ffi::c_uint,
        num: i64,
        a1: u64,
        a2: u64,
        a3: u64,
        a4: u64,
        a5: u64,
        a6: u64,
        a7: u64,
        a8: u64,
    ),
>;
extern "C" {
    pub fn qemu_plugin_register_vcpu_syscall_cb(
        id: QemuPluginId,
        cb: QemuPluginVcpuSyscallCb,
    );
}
pub type QemuPluginVcpuSyscallRetCb = ::core::option::Option<
    unsafe extern "C" fn(id: QemuPluginId, vcpu_idx: ::core::ffi::c_uint, num: i64, ret: i64),
>;
extern "C" {
    pub fn qemu_plugin_register_vcpu_syscall_ret_cb(
        id: QemuPluginId,
        cb: QemuPluginVcpuSyscallRetCb,
    );
}
extern "C" {
    #[doc = " qemu_plugin_insn_disas() - return disassembly string for instruction\n @insn: instruction reference\n\n Returns an allocated string containing the disassembly"]
    pub fn qemu_plugin_insn_disas(insn: *const QemuPluginInsn) -> *mut ::core::ffi::c_char;
}
extern "C" {
    #[doc = " qemu_plugin_insn_symbol() - best effort symbol lookup\n @insn: instruction reference\n\n Return a static string referring to the symbol. This is dependent\n on the binary QEMU is running having provided a symbol table."]
    pub fn qemu_plugin_insn_symbol(insn: *const QemuPluginInsn) -> *const ::core::ffi::c_char;
}
extern "C" {
    #[doc = " qemu_plugin_vcpu_for_each() - iterate over the existing vCPU\n @id: plugin ID\n @cb: callback function\n\n The @cb function is called once for each existing vCPU.\n\n See also: qemu_plugin_register_vcpu_init_cb()"]
    pub fn qemu_plugin_vcpu_for_each(id: QemuPluginId, cb: QemuPluginVcpuSimpleCb);
}
extern "C" {
    pub fn qemu_plugin_register_flush_cb(id: QemuPluginId, cb: QemuPluginSimpleCb);
}
extern "C" {
    #[doc = " qemu_plugin_register_atexit_cb() - register exit callback\n @id: plugin ID\n @cb: callback\n @userdata: user data for callback\n\n The @cb function is called once execution has finished. Plugins\n should be able to free all their resources at this point much like\n after a reset/uninstall callback is called.\n\n In user-mode it is possible a few un-instrumented instructions from\n child threads may run before the host kernel reaps the threads."]
    pub fn qemu_plugin_register_atexit_cb(
        id: QemuPluginId,
        cb: QemuPluginUdataCb,
        userdata: *mut ::core::ffi::c_void,
    );
}
extern "C" {
    pub fn qemu_plugin_n_vcpus() -> ::core::ffi::c_int;
}
extern "C" {
    pub fn qemu_plugin_n_max_vcpus() -> ::core::ffi::c_int;
}
extern "C" {
    #[doc = " qemu_plugin_outs() - output string via QEMU's logging system\n @string: a string"]
    pub fn qemu_plugin_outs(string: *const ::core::ffi::c_char);
}
extern "C" {
    #[doc = " qemu_plugin_bool_parse() - parses a boolean argument in the form of\n \"<argname>=[on|yes|true|off|no|false]\"\n\n @name: argument name, the part before the equals sign\n @val: argument value, what's after the equals sign\n @ret: output return value\n\n returns true if the combination @name=@val parses correctly to a boolean\n argument, and false otherwise"]
    pub fn qemu_plugin_bool_parse(
        name: *const ::core::ffi::c_char,
        val: *const ::core::ffi::c_char,
        ret: *mut bool,
    ) -> bool;
}
extern "C" {
    #[doc = " qemu_plugin_path_to_binary() - path to binary file being executed\n\n Return a string representing the path to the binary. For user-mode\n this is the main executable. For system emulation we currently\n return NULL. The user should g_free() the string once no longer\n needed."]
    pub fn qemu_plugin_path_to_binary() -> *const ::core::ffi::c_char;
}
extern "C" {
    #[doc = " qemu_plugin_start_code() - returns start of text segment\n\n Returns the nominal start address of the main text segment in\n user-mode. Currently returns 0 for system emulation."]
    pub fn qemu_plugin_start_code() -> u64;
}
extern "C" {
    #[doc = " qemu_plugin_end_code() - returns end of text segment\n\n Returns the nominal end address of the main text segment in\n user-mode. Currently returns 0 for system emulation."]
    pub fn qemu_plugin_end_code() -> u64;
}
extern "C" {
    #[doc = " qemu_plugin_entry_code() - returns start address for module\n\n Returns the nominal entry address of the main text segment in\n user-mode. Currently returns 0 for system emulation."]
    pub fn qemu_plugin_entry_code() -> u64;
}
extern "C" {
    #[doc = " qemu_plugin_vcpu_memory_rw() - reads or writes guest's virtual or physicalmemory\n\n @vcpu_index: vcpu index\n @addr: guest's address\n @buf: data buffer\n @len: number of bytes to transfer\n @is_write: whether to read from buf or write to buf\n @is_phys: whether to interpret addr as virtual or physical address"]
    pub fn qemu_plugin_vcpu_memory_rw(
        vcpu_index: ::core::ffi::c_uint,
        addr: u64,
        buf: *mut ::core::ffi::c_void,
        len: u64,
        is_write: bool,
        is_phys: bool,
    );
}
