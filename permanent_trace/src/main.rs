use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use clap::{Parser, Subcommand};
use permanent_common::config::{VmConfig, TestConfig, TraceConfig, TraceType};

mod pipe;
mod vm;
mod tracer;

fn remove_dir(path: &String) -> Result<(), std::io::Error> {
    if Path::new(path).exists() {
        std::fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn read_vm_config(work_dir: &String) -> VmConfig {
    let vm_config_file = File::open(format!("{}/vm_config.yaml", work_dir).as_str()).expect("Could not open vm config file");
    serde_yaml::from_reader(BufReader::new(vm_config_file)).expect("Could not deserialize vm config file")
}

fn read_test_config(work_dir: &String) -> TestConfig {
    let test_config_file = File::open(format!("{}/test_config.yaml", work_dir).as_str()).expect("Could not open test config file");
    serde_yaml::from_reader(BufReader::new(test_config_file)).expect("Could not deserialize test config file")
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Analyse { work_dir, force } => {
            let vm_config = read_vm_config(&work_dir);
            let test_config = read_test_config(&work_dir);
            let trace_config = TraceConfig::new(&work_dir, TraceType::Analyse);

            if force {
                remove_dir(&trace_config.trace_dir()).unwrap();
            }
            std::fs::create_dir(trace_config.trace_dir()).expect("could not create trace dir");
            tracer::trace_vm(&work_dir, &vm_config, Some(&test_config), &trace_config);
        },
        Command::PostSuccess { work_dir, pmem_hash, nvme_hash, force } => {
            todo!();
        },
        Command::PostFailure { work_dir, pmem_hash, nvme_hash, force } => {
                let vm_config = read_vm_config(&work_dir);
                let test_config = read_test_config(&work_dir);
            let trace_config = TraceConfig::new(&work_dir, TraceType::PostFailure { pmem_hash, nvme_hash });

            if force {
                remove_dir(&trace_config.trace_dir()).unwrap();
            }
            std::fs::create_dir(trace_config.trace_dir()).expect("could not create trace dir");
            tracer::trace_vm(&work_dir, &vm_config, Some(&test_config), &trace_config);
        }
    }
}

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Analyse {
        work_dir: String,
        #[clap(short, long, action)]
        force: bool,
    },
    PostSuccess {
        work_dir: String,
        #[arg(short, long)]
        pmem_hash: Option<String>,
        #[arg(short, long)]
        nvme_hash: Option<String>,
        #[clap(short, long, action)]
        force: bool,
    },
    PostFailure {
        work_dir: String,
        #[arg(short, long)]
        pmem_hash: Option<String>,
        #[arg(short, long)]
        nvme_hash: Option<String>,
        #[clap(short, long, action)]
        force: bool,
    }
}
