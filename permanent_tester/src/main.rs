use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::process::Command;
use std::collections::{HashMap, HashSet};
use clap::Parser;
use itertools::Itertools;
use permanent_common::config::{VmConfig, TestConfig, TraceConfig, TraceType};

const START_MSG: &'static str = "PERMANENT START";
const END_MSG: &'static str = "PERMANENT END";
const SUCCESS_MSG: &'static str = "PERMANENT SUCCESS";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct StateHash(blake3::Hash);

impl serde::Serialize for StateHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        serializer.serialize_str(&self.0.to_hex())
    }
}

fn main() {
    let args = Args::parse();

    let vm_config_file = File::open(format!("{}/vm_config.yaml", args.work_dir).as_str()).expect("Could not open vm config file");
    let vm_config: VmConfig = serde_yaml::from_reader(BufReader::new(vm_config_file)).expect("Could not deserialize vm config file");

    std::fs::create_dir(format!("{}/states", args.work_dir).as_str()).expect("could not create states dir");
    let mut state_hashes: HashMap<StateHash, Vec<String>> = HashMap::new();

    let (p, n) = vm_config.have_pmem_nvme();
    if p && n {
        // TODO CrashHash instead of String
        let pmem_index: HashMap<usize, HashSet<String>> = serde_json::from_reader(
            BufReader::new(File::open(format!("{}/pmem.index", args.work_dir).as_str()).unwrap())
        ).unwrap();
        let nvme_index: HashMap<usize, HashSet<String>> = serde_json::from_reader(
            BufReader::new(File::open(format!("{}/nvme.index", args.work_dir).as_str()).unwrap())
        ).unwrap();
        let mut gen_indices_pmem: Vec<usize> = pmem_index.keys().copied().collect();
        gen_indices_pmem.sort();
        let mut gen_indices_nvme: Vec<usize> = nvme_index.keys().copied().collect();
        gen_indices_nvme.sort();
        if gen_indices_pmem != gen_indices_nvme {
            panic!("index file key discrepancy");
        }
        let gen_indices = gen_indices_pmem;

        let total_amount: usize = gen_indices.iter()
            .map(|id| pmem_index.get(id).unwrap().len() * nvme_index.get(id).unwrap().len())
            .sum();
        let mut c = 0;
        let mut seen: HashSet<(String, String)> = HashSet::new();

        for id in gen_indices {
            let pmem_hashes = pmem_index.get(&id).unwrap();
            let nvme_hashes = nvme_index.get(&id).unwrap();
            for (pmem_hash, nvme_hash) in pmem_hashes.iter().cartesian_product(nvme_hashes.iter()) {
                c += 1;
                let combination = (pmem_hash.clone(), nvme_hash.clone());
                if seen.contains(&combination) {
                    println!("[{}/{}] trace {} {} ... already done.", c, total_amount, pmem_hash, nvme_hash);
                    continue;
                }
                seen.insert(combination);
                println!("[{}/{}] trace {} {}", c, total_amount, pmem_hash, nvme_hash);

                let dir = trace_dir(&args.work_dir, Some(pmem_hash), Some(nvme_hash));
                let success = Command::new("target/release/permanent_trace")
                    .arg("post-failure")
                    .arg(args.work_dir.as_str())
                    .args(["--pmem-hash", pmem_hash])
                    .args(["--nvme-hash", nvme_hash])
                    .spawn()
                    .expect("could not start permanent_trace")
                    .wait()
                    .expect("could not collect permanent_trace process")
                    .success();
                if !success {
                    eprintln!("WARNING: trace {} {} returned non-zero exit status. skipped.", pmem_hash, nvme_hash);
                } else {
                    let log = std::fs::read(format!("{}/log", dir).as_str()).expect("could not read log");
                    let success = log.windows(SUCCESS_MSG.len()).any(|win| win == SUCCESS_MSG.as_bytes());
                    let state_dump = if success {
                        extract_state_dump(log.as_slice())
                    } else {
                        b"FAILED"
                    };
                    let state_hash = StateHash(blake3::hash(state_dump));
                    let state_hash_string = state_hash.0.to_hex();
                    let crash_hashes = state_hashes.entry(state_hash).or_insert(Vec::new());
                    if crash_hashes.is_empty() {
                        let mut f = File::create(format!("{}/states/{}.state", args.work_dir, state_hash_string).as_str())
                            .expect("could not create state file");
                        f.write_all(state_dump).expect("could not write state file");
                    }
                    crash_hashes.push(format!("{}_{}", pmem_hash, nvme_hash));
                }
                clean_dir(&dir);
            }
        }


    } else if p || n {
        // TODO total_amount
        let hash_type_arg = if p { "--pmem-hash" } else { "--nvme-hash" };
        for path in std::fs::read_dir(format!("{}/crash_images", args.work_dir).as_str())
            .expect("could not read crash_image dir")
        {
            let filename = path.unwrap().file_name();
            let pathref: &Path = filename.as_ref();
            let crash_hash: String = pathref.file_stem().unwrap().to_str().unwrap().to_string();
            let dir = if p {
                trace_dir(&args.work_dir, Some(&crash_hash), None)
            } else {
                trace_dir(&args.work_dir, None, Some(&crash_hash))
            };
            let success = Command::new("target/release/permanent_trace") // TODO release
                .arg("post-failure")
                .arg(args.work_dir.as_str())
                .args([hash_type_arg, crash_hash.as_str()])
                .spawn()
                .expect("could not start permanent_trace")
                .wait()
                .expect("could not collect permanent_trace process")
                .success();
            if !success {
                eprintln!("WARNING: permanent_trace {} returned non-zero exit status. skipped.", crash_hash);
            } else {
                let log = std::fs::read(format!("{}/log", dir).as_str()).expect("could not read log");
                let success = log.windows(SUCCESS_MSG.len()).any(|win| win == SUCCESS_MSG.as_bytes());
                let state_dump = if success {
                    extract_state_dump(log.as_slice())
                } else {
                    b"FAILED"
                };
                let state_hash = StateHash(blake3::hash(state_dump));
                let state_hash_string = state_hash.0.to_hex();
                let crash_hashes = state_hashes.entry(state_hash).or_insert(Vec::new());
                if crash_hashes.is_empty() {
                    let mut f = File::create(format!("{}/states/{}.state", args.work_dir, state_hash_string).as_str())
                        .expect("could not create state file");
                    f.write_all(state_dump).expect("could not write state file");
                }
                crash_hashes.push(crash_hash);
            }
            clean_dir(&dir);
        }
    } else {
        unreachable!();
    }
    let out_file = File::create(format!("{}/states.index", args.work_dir).as_str()).expect("could not create output file");
    serde_json::to_writer_pretty(BufWriter::new(out_file), &state_hashes).expect("could not write output");
}

fn extract_state_dump(data: &[u8]) -> &[u8] {
    let start_pos = data.windows(START_MSG.len()).position(|win| win == START_MSG.as_bytes()).expect("no START")
        + START_MSG.len();
    let end_pos = data.windows(END_MSG.len()).position(|win| win == END_MSG.as_bytes()).expect("no END");
    &data[start_pos..end_pos]
}

// TODO this is already implemented in permanent_common
fn trace_dir(work_dir: &String, pmem_hash: Option<&String>, nvme_hash: Option<&String>) -> String {
    let mut dir = format!("{}/post", work_dir);
    if let Some(hash) = pmem_hash {
        dir.push('_');
        dir.push_str(hash.as_str());
    }
    if let Some(hash) = nvme_hash {
        dir.push('_');
        dir.push_str(hash.as_str());
    }
    dir
}

// remove everything except logs for debugging
fn clean_dir(dir: &String) {
    let files = ["trace.bin", "pmem.raw", "nvme.raw", "pipe.in", "pipe.out"];
    for file in files {
        let file = format!("{}/{}", dir, file);
        if Path::new(file.as_str()).exists() {
            if let Err(_) = std::fs::remove_file(file.as_str()) {
                eprintln!("WARNING: could not remove {}", file);
            }
        }
    }
}

#[derive(Debug, Parser)]
pub struct Args {
    work_dir: String,
}
