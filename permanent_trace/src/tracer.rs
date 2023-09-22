use std::time::SystemTime;

use permanent_common::config::{VmConfig, TestConfig, TraceConfig, TraceType};
use permanent_common::profiler::Measurement;

use crate::vm::VM;
use crate::pipe::Pipe;

pub fn trace_vm(work_dir: &String, vm_config: &VmConfig, test_config: Option<&TestConfig>, trace_config: &TraceConfig) {
    // TODO we always copy, even on PostSuccess.
    // might be a little inefficient, but we can't risk the post recovery to write to NVME.
    // it also writes on mount -oro, in case of recovery.
    let (p, n) = vm_config.have_pmem_nvme();
    match &trace_config.trace_type {
        TraceType::Analyse => {
            if p {
                std::fs::copy(format!("{}/pmem_base.raw", work_dir).as_str(),
                              trace_config.pmem_image_path().as_str())
                    .expect("could not copy base image");
            }
            if n {
                std::fs::copy(format!("{}/nvme_base.raw", work_dir).as_str(),
                              trace_config.nvme_image_path().as_str())
                    .expect("could not copy base image");
            }
        },
        TraceType::PostSuccess => { todo!(); },
        TraceType::PostFailure { pmem_hash, nvme_hash } => {
            if p {
                std::fs::copy(format!("{}/crash_images/{}.raw", work_dir, pmem_hash.as_ref().unwrap()).as_str(),
                              trace_config.pmem_image_path().as_str())
                    .expect("could not copy base image");
            }
            if n {
                std::fs::copy(format!("{}/crash_images/{}.raw", work_dir, nvme_hash.as_ref().unwrap()).as_str(),
                              trace_config.nvme_image_path().as_str())
                    .expect("could not copy base image");
            }
        }
    }

    // 2. create pipe
    Pipe::make(&trace_config.pipe_path()).expect("Could not create control pipe");

    // 3. init vm & wait for startup
    let mut vm = VM::init(&vm_config, &trace_config);

    // 4. run tests & wait for end
    let text = match &trace_config.trace_type {
        TraceType::Analyse => {
            format!("(checkpoint 255 && {} && {} && checkpoint success) || checkpoint fail\n", &vm_config.trace_cmd_prefix, test_config.unwrap().trace_cmd_suffix)
        },
        TraceType::PostSuccess => {
            format!("(checkpoint 255 && {} && checkpoint success) || checkpoint fail\n", &vm_config.recovery_cmd)
        },
        TraceType::PostFailure { .. } => {
            format!("(checkpoint 255 && {} && {} && checkpoint success) || checkpoint fail\n", &vm_config.dump_cmd_prefix, &test_config.unwrap().dump_cmd_suffix)
        },
    };
    println!("== sh command: {}", text);
    vm.send(text.as_str()).unwrap();

    // 5. shutdown vm
    let success = vm.teardown();
    if let TraceType::Analyse = &trace_config.trace_type {
        if !success {
            panic!("trace analyse was not successful! try tracing with a different shell command");
        }
    }
}
