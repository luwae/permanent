use std::collections::{HashSet, HashMap};
use std::io::{BufReader, BufWriter};
use std::fs::File;
use std::time::SystemTime;
use std::marker::PhantomData;
use anyhow::{bail, Result};
use permanent_common::action::Action;
use permanent_common::config::{VmConfig, TestConfig};
use permanent_common::profiler::{Profile, Measurement};
use permanent_common::trace::{TraceEntry, PmemEvent, NvmeEvent, parse_trace_file_bin};

mod set;

mod image;
use image::{CrashHash, ImagePool};

mod models;
use models::{X86PersistentMemory, NvmeDevice};

enum CrashPersistenceType {
    NoWrites,
    NothingPersisted,
    FullyPersisted, // { /* TODO */ },
    StrictSubsetPersisted, // { /* TODO */ }
}

struct CrashMetadata {
    trace_entry_id: usize,
    prev_checkpoint_value: Option<u8>,
    persistence_type: CrashPersistenceType,
}

// TODO has_changed optimization
struct DeviceData<D> {
    device: D,
    changed: bool,
    last_generated_index: Option<usize>,
    generated: HashMap<usize, HashSet<CrashHash>>,
    // metadata: HashMap<blake3::Hash, Vec<CrashMetadata>>,
}

pub struct CrashImageGenerator {
    work_dir: String,
    vm_config: VmConfig,
    test_config: TestConfig,

    pool: ImagePool,
    pmem: Option<DeviceData<X86PersistentMemory>>,
    nvme: Option<DeviceData<NvmeDevice>>,
    rng: fastrand::Rng,
}

const POOL_LIMIT: usize = 20*1024*1024*1024;

impl CrashImageGenerator {
    pub fn new(work_dir: &String, vm_config: &VmConfig, test_config: &TestConfig) -> Self {
        let (p, n) = vm_config.have_pmem_nvme();
        Self {
            work_dir: work_dir.clone(),
            vm_config: vm_config.clone(),
            test_config: test_config.clone(),

            pool: ImagePool::with_limit(work_dir, POOL_LIMIT).unwrap(),
            pmem: p.then(|| DeviceData {
                device: X86PersistentMemory::new(std::fs::read(format!("{}/pmem_base.raw", &work_dir).as_str()).unwrap()),
                changed: true,
                last_generated_index: None,
                generated: HashMap::new(),
            }),
            nvme: n.then(|| DeviceData {
                device: NvmeDevice::new(std::fs::read(format!("{}/nvme_base.raw", &work_dir).as_str()).unwrap()),
                changed: true,
                last_generated_index: None,
                generated: HashMap::new(),
            }),
            rng: fastrand::Rng::new(),
        }
    }

    fn generate_crash_images_at(&mut self, trace_entry_id: usize) {
        println!("generate crash images at id {}", trace_entry_id);
        let (p, n) = self.vm_config.have_pmem_nvme();
        if p {
            if self.get_pmem_mut().changed {
                let nothing_hash = self.pmem.as_ref().unwrap().device.generate_nothing_persisted_image(&mut self.pool);
                let everything_hash = self.pmem.as_ref().unwrap().device.generate_everything_persisted_image(&mut self.pool);
                let mut hashes = self.pmem.as_ref().unwrap().device.generate_random_images(&mut self.pool, &mut self.rng);
                hashes.insert(nothing_hash);
                hashes.insert(everything_hash);
                self.get_pmem_mut().changed = false;
                self.get_pmem_mut().last_generated_index = Some(trace_entry_id);
                self.get_pmem_mut().generated.insert(trace_entry_id, hashes.clone());
            } else {
                // reuse last set of images
                let last_index = self.pmem.as_ref().unwrap().last_generated_index.expect("no last_generated_index");
                let last_images = self.pmem.as_ref().unwrap().generated.get(&last_index)
                        .expect("last_generated_index hashes not found")
                        .clone();
                self.get_pmem_mut().generated.insert(trace_entry_id, last_images);
            }
        }
        if n {
            if self.get_nvme_mut().changed {
                let nothing_hash = self.nvme.as_ref().unwrap().device.generate_nothing_persisted_image(&mut self.pool);
                let everything_hash = self.nvme.as_ref().unwrap().device.generate_everything_persisted_image(&mut self.pool);
                let mut hashes = self.nvme.as_ref().unwrap().device.generate_random_images(&mut self.pool, &mut self.rng);
                hashes.insert(nothing_hash);
                hashes.insert(everything_hash);
                self.get_nvme_mut().changed = false;
                self.get_nvme_mut().last_generated_index = Some(trace_entry_id);
                self.get_nvme_mut().generated.insert(trace_entry_id, hashes.clone());
            } else {
                // reuse last set of images
                let last_index = self.nvme.as_ref().unwrap().last_generated_index.expect("no last_generated_index");
                let last_images = self.nvme.as_ref().unwrap().generated.get(&last_index)
                        .expect("last_generated_index hashes not found")
                        .clone();
                self.get_nvme_mut().generated.insert(trace_entry_id, last_images);
            }
        }
    }
    
    fn get_pmem_mut(&mut self) -> &mut DeviceData<X86PersistentMemory> {
        self.pmem.as_mut().unwrap()
    }

    fn get_nvme_mut(&mut self) -> &mut DeviceData<NvmeDevice> {
        self.nvme.as_mut().unwrap()
    }

    pub fn replay_trace(&mut self) { // TODO use anyhow results
        let mut had_init = false;
        let mut prev_checkpoint_value: Option<u8> = None;
        let mut checkpoint_ids: HashMap<u8, usize> = HashMap::new();

        let checkpoint_range = self.test_config.checkpoint_range.0
            .. self.test_config.checkpoint_range.1;
        let within_checkpoint_range = |maybe_value: Option<u8>| { maybe_value.is_some_and(|value| checkpoint_range.contains(&value)) };

        // TODO path
        let trace_file = File::open(format!("{}/analyse/trace.bin", self.work_dir).as_str())
            .expect("could not open trace file");
        for entry in parse_trace_file_bin(BufReader::new(trace_file)) {
            match entry.unwrap() {
                TraceEntry::Pmem { id, event } => {
                    match event {
                        PmemEvent::Read  { .. } => { },
                        PmemEvent::Write { address, size: _, content, non_temporal } => {
                            if !had_init {
                                panic!("pmem event before test script");
                            }
                            self.get_pmem_mut().changed = true;
                            self.get_pmem_mut().device.write(address as usize, content.as_slice(), non_temporal);
                        },
                        PmemEvent::Clflush { address } => {
                            if !had_init {
                                panic!("pmem event before test script");
                            }
                            self.get_pmem_mut().device.clwb(address as usize, None);
                            // approximate Clflush by adding a fence
                            // this necessitates crash image generation.
                            if within_checkpoint_range(prev_checkpoint_value) {
                                // we do not generate crash images before the first or after the
                                // last checkpoint.
                                if !self.get_pmem_mut().device.pending_lines.is_empty() {
                                    self.generate_crash_images_at(id as usize);
                                    self.get_pmem_mut().changed = true; // after a fence with flushes, different
                                                         // crash images are possible
                                }
                            }
                            self.get_pmem_mut().device.fence();
                        },
                        PmemEvent::Clflushopt { address } => {
                            if !had_init {
                                panic!("pmem event before test script");
                            }
                            self.get_pmem_mut().device.clwb(address as usize, None);
                        },
                        PmemEvent::Clwb { address } => {
                            if !had_init {
                                panic!("pmem event before test script");
                            }
                            self.get_pmem_mut().device.clwb(address as usize, None);
                        },
                        PmemEvent::Wbinvd => {
                            if had_init { // yes, there should be no ! here.
                                panic!("wbinvd should not appear during normal test execution")
                            }
                        },
                        PmemEvent::Fence => {
                            if within_checkpoint_range(prev_checkpoint_value) {
                                // we do not generate crash images before the first or after the
                                // last checkpoint.
                                if !self.get_pmem_mut().device.pending_lines.is_empty() {
                                    self.generate_crash_images_at(id as usize);
                                    self.get_pmem_mut().changed = true; // after a fence with flushes, different
                                                         // crash images are possible
                                }
                            }
                            self.get_pmem_mut().device.fence();
                        },
                    }
                },
                TraceEntry::Nvme { id, event } => {
                    match event {
                        NvmeEvent::Read { .. } => { },
                        NvmeEvent::Write { offset, length: _, data } => {
                            if !had_init {
                                panic!("nvme event before test script");
                            }
                            self.get_nvme_mut().changed = true;
                            self.get_nvme_mut().device.write(offset as usize, data);
                        }
                        NvmeEvent::Flush => {
                            if !had_init {
                                panic!("nvme event before test script");
                            }
                            if within_checkpoint_range(prev_checkpoint_value) {
                                if !self.get_nvme_mut().device.unpersisted_content.is_empty() {
                                    self.generate_crash_images_at(id as usize);
                                    self.get_nvme_mut().changed = true; // after flush with writes, different
                                                         // crash images are possible
                                }
                            }
                            self.get_nvme_mut().device.flush();
                        },
                    }
                },
                TraceEntry::Checkpoint { id, value } => {
                    if value == 255 {
                        had_init = true;
                    } else {
                        prev_checkpoint_value = Some(value);
                        if value > 0 && !checkpoint_ids.contains_key(&(value - 1)) { // TODO do we want this?
                            panic!("non-contiguous checkpoints; missing: {}", value - 1);
                        }
                        if checkpoint_ids.insert(value, id as usize).is_some() {
                            panic!("duplicate checkpoint value: {}", value);
                        }
                        // we create crash images at every checkpoint including the last (for SFS)
                        if within_checkpoint_range(prev_checkpoint_value) || self.test_config.checkpoint_range.1 == value {
                            self.generate_crash_images_at(id as usize);
                        }
                    }
                },
            }
        }
        
        if !checkpoint_ids.contains_key(&self.test_config.checkpoint_range.1) {
            panic!("ERROR: not all checkpoints are present in the trace. abort.")
        }

        // write index information
        let (p, n) = self.vm_config.have_pmem_nvme();
        if p {
            let pmem = self.pmem.as_ref().unwrap();
            let file = File::create(format!("{}/pmem.index", self.work_dir).as_str()).unwrap();
            serde_json::to_writer_pretty(BufWriter::new(file), &pmem.generated).unwrap();
        }
        if n {
            let nvme = self.nvme.as_ref().unwrap();
            let file = File::create(format!("{}/nvme.index", self.work_dir).as_str()).unwrap();
            serde_json::to_writer_pretty(BufWriter::new(file), &nvme.generated).unwrap();
        }
        // write checkpoint information
        let file = File::create(format!("{}/checkpoint.index", self.work_dir).as_str()).unwrap();
        serde_json::to_writer_pretty(BufWriter::new(file), &checkpoint_ids).unwrap();
    }
}
