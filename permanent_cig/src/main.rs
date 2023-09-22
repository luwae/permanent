use std::io::BufReader;
use std::fs::File;
use clap::Parser;
use std::path::Path;
use permanent_common::action::Action;
use permanent_common::config::{VmConfig, TestConfig};
use permanent_cig::CrashImageGenerator;

fn remove_dir(path: &String) -> Result<(), std::io::Error> {
    if Path::new(path).exists() {
        std::fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn remove_file(path: &String) -> Result<(), std::io::Error> {
    if Path::new(path).exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn main() {
    let args = Args::parse();
    let vm_config: VmConfig = serde_yaml::from_reader(BufReader::new(File::open(format!("{}/vm_config.yaml", args.work_dir).as_str()).unwrap())).unwrap();
    let test_config: TestConfig = serde_yaml::from_reader(BufReader::new(File::open(format!("{}/test_config.yaml", args.work_dir).as_str()).unwrap())).unwrap();

    let make_path = |suffix| format!("{}/{}", args.work_dir, suffix);

    if args.force {
        remove_dir(&make_path("crash_images")).unwrap();
        remove_file(&make_path("pmem.index")).unwrap();
        remove_file(&make_path("nvme.index")).unwrap();
        remove_file(&make_path("checkpoint.index")).unwrap();
    }
    let mut cig = CrashImageGenerator::new(&args.work_dir, &vm_config, &test_config);
    cig.replay_trace();
}

#[derive(Debug, Parser)]
pub struct Args {
    work_dir: String,
    #[clap(short, long, action)]
    force: bool,
}
