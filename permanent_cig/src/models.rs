use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::marker::Sized;
use itertools::Itertools;

use permanent_common::trace::{PmemEvent, NvmeEvent};
use crate::image::{ImagePool, CrashHash};
use crate::set;

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Store {
    pub address: usize,
    pub data: Vec<u8>,
}

impl Store {
    pub fn address_start(&self) -> usize {
        self.address
    }

    pub fn address_end(&self) -> usize {
        self.address + self.data.len()
    }

    pub fn address_range(&self) -> Range<usize> {
        self.address_start()..self.address_end()
    }
}

#[derive(Clone)]
pub struct OrderedWriteLine {
    writes: Vec<Store>,
    /// everything up until but excluding this index has been marked for flushing
    flushed_index: usize,
}

impl Default for OrderedWriteLine {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderedWriteLine {
    pub fn new() -> Self {
        OrderedWriteLine {
            writes: Vec::new(),
            flushed_index: 0,
        }
    }

    pub fn flush_all(&mut self) {
        self.flushed_index = self.writes.len();
    }

    pub fn all_writes(&self) -> &[Store] {
        &self.writes
    }

    pub fn flushed_writes(&self) -> &[Store] {
        &self.writes[0..self.flushed_index]
    }

    pub fn unflushed_writes(&self) -> &[Store] {
        &self.writes[self.flushed_index..]
    }

    pub fn drain_flushed_writes(&mut self) -> std::vec::Drain<'_, Store> {
        let idx = self.flushed_index;
        self.flushed_index = 0;
        self.writes.drain(0..idx)
    }
    
    /// Do any pending writes overlap with an access at the specified address and size?
    pub fn overlaps_access(&self, address: usize, size: usize) -> bool {
        let access_range = address..(address + size);
        self.writes
            .iter()
            .any(|w| !range_overlap(&w.address_range(), &access_range).is_empty())
    }
}

/// x86 memory persistency model.
///
/// writes to the same cache line are always ordered in respect to each other.
/// writes to different cache lines may be reordered.
pub struct X86PersistentMemory {
    pub persisted_content: Vec<u8>,
    pub pending_lines: HashSet<usize>,
    /// maps line number (== address / line_granularity) to OrderedWriteLine
    pub unpersisted_content: HashMap<usize, OrderedWriteLine>,
    /// 8 or 64
    line_granularity: usize,
}

// TODO
const LINE_GRANULARITY: usize = 64;
const MAX_UNPERSISTED_SUBSETS: usize = 5;
const MAX_PARTIAL_FLUSHES_COUNT: usize = 5;

impl X86PersistentMemory {
    pub fn new(persisted_content: Vec<u8>) -> Self {
        Self {
            persisted_content,
            pending_lines: HashSet::new(),
            unpersisted_content: HashMap::new(),
            line_granularity: LINE_GRANULARITY,
        }
    }

    pub fn generate_nothing_persisted_image(&self, pool: &mut ImagePool) -> CrashHash {
        let (_, hash) = pool.persist(self.persisted_content.as_slice()).unwrap();
        hash
    }

    pub fn generate_everything_persisted_image(&self, pool: &mut ImagePool) -> CrashHash {
        let mut img: Vec<u8> = self.persisted_content.clone();
        for ordered_write_line in self.unpersisted_content.values() {
            for store in ordered_write_line.all_writes().iter() {
                img[store.address_range()].copy_from_slice(store.data.as_slice());
            }
        }
        let (_, hash) = pool.persist(img.as_slice()).unwrap();
        hash
    }

    pub fn generate_random_images(&self, pool: &mut ImagePool, rng: &mut fastrand::Rng) -> HashSet<CrashHash> {
        let mut img: Vec<u8> = vec![0u8; self.persisted_content.len()];
        let mut hashes = HashSet::new();

        let unpersisted_reads_lines: Vec<usize> = self.unpersisted_content.keys().copied().collect(); // TODO heuristic
        if !unpersisted_reads_lines.is_empty() {
            let random_subsets: Vec<Vec<usize>> = if 1usize.checked_shl(unpersisted_reads_lines.len().try_into().unwrap())
                .is_some_and(|val| val <= (MAX_UNPERSISTED_SUBSETS + 1).try_into().unwrap())
            {
                unpersisted_reads_lines
                    .iter()
                    .copied()
                    .powerset()
                    .skip(1) // empty set
                    .collect()
            } else {
                set::random_subsets(rng, &unpersisted_reads_lines)
                    .filter(|vec| !vec.is_empty())
                    .take(MAX_UNPERSISTED_SUBSETS)
                    .collect()
            };
            for random_lines in random_subsets {
                let partial_flushes_count = random_lines
                    .iter()
                    .map(|line_number| self.unpersisted_content[line_number].all_writes().len())
                    .fold(1, |acc, x| acc * x);
                let line_partial_writes: Vec<Vec<usize>> = random_lines
                    .iter()
                    .map(|line_number| {
                        let writes_count = self.unpersisted_content[line_number].all_writes().len();
                        if partial_flushes_count > MAX_PARTIAL_FLUSHES_COUNT {
                            if writes_count <= 1 {
                                vec![writes_count]
                            } else {
                                vec![writes_count, rng.usize(1..writes_count)]
                            }
                        } else {
                            (1..=writes_count).collect()
                        }
                    })
                    .collect();
                for partial_writes_indices in line_partial_writes.iter().multi_cartesian_product()
                {
                    img[..].copy_from_slice(self.persisted_content.as_slice());
                    for (line_number, flush_writes_limit) in random_lines
                        .iter()
                        .copied()
                        .zip(partial_writes_indices.iter().copied())
                    {
                        for store in self.unpersisted_content[&line_number].all_writes().iter().take(*flush_writes_limit) {
                            img[store.address_range()].copy_from_slice(store.data.as_slice());
                        }
                        let (_, hash) = pool.persist(img.as_slice()).unwrap();
                        hashes.insert(hash);
                    }
                }
            }
        }
        hashes
    }

    pub fn write(&mut self, address: usize, value: &[u8], non_temporal: bool) {
        // test to see if we even get larger stores
        assert!(matches!(value.len(), 1 | 2 | 4 | 8));
        let address_stop = address + value.len();
        let split_address_ranges = {
            let start = address - address % 8;
            let stop = if address_stop % 8 == 0 {
                address_stop
            } else {
                address_stop + 8 - (address_stop % 8)
            };
            (start..stop)
                .step_by(8)
                .map(|a| max(a, address)..min(a + 8, address_stop))
        };

        for address_range in split_address_ranges {
            let line_number = address_range.start / self.line_granularity;
            let line = self
                .unpersisted_content
                .entry(line_number)
                .or_insert_with(OrderedWriteLine::new);
            line.writes.push(Store {
                address: address_range.start,
                data: value[(address_range.start - address)..(address_range.end - address)].into(),
            });

            // approximation of non-temporal stores
            if non_temporal {
                self.pending_lines.insert(line_number);
                // note that for cache line granularity, this is probably not quite correct
                line.flush_all();
            }
        }
    }

    // TODO: what do we need flush_writes_limit for?
    pub fn clwb(&mut self, address: usize, flush_writes_limit: Option<usize>) {
        let cache_line_base = (address >> 6) << 6;
        for a in (cache_line_base..(cache_line_base + 64)).step_by(self.line_granularity) {
            let line_number = a / self.line_granularity;
            if let Some(line) = self.unpersisted_content.get_mut(&line_number) {
                self.pending_lines.insert(line_number);
                if let Some(limit) = flush_writes_limit {
                    line.flushed_index = limit;
                } else {
                    line.flush_all();
                }
            }
        }
    }

    pub fn fence(&mut self) {
        // A fence consumes all pending lines. Swap in a new set to avoid double borrow of self.
        let mut pending_lines = HashSet::new();
        std::mem::swap(&mut pending_lines, &mut self.pending_lines);
        for line in pending_lines {
            self.fence_line(line);
        }
    }

    fn fence_line(&mut self, line: usize) {
        if let Some(content) = self.unpersisted_content.get_mut(&line) {
            assert!(content.flushed_index > 0);
            for write in content.drain_flushed_writes() {
                self.persisted_content[write.address_range()].copy_from_slice(&write.data);
            }
            if content.writes.is_empty() {
                self.unpersisted_content.remove(&line);
            }
            self.pending_lines.remove(&line);
        } else {
            unreachable!();
        }
    }

    pub fn persist_unpersisted(&mut self) {
        let lines: Vec<usize> = self.unpersisted_content.keys().copied().collect();
        for line_number in lines {
            self.clwb(line_number * self.line_granularity, None);
        }
        self.fence();
        assert!(self.unpersisted_content.is_empty());
        assert!(self.pending_lines.is_empty());
    }

    pub fn print_unpersisted(&self) {
        let mut lines: Vec<(&usize, &OrderedWriteLine)> = self.unpersisted_content.iter().collect();
        lines.sort_by_key(|(line_number, _)| *line_number);
        for (line_number, line) in lines {
            println!("unpersisted line {}: {:?}", *line_number, line.writes);
        }
    }
}

/// nvme persistency model.
///
/// all writes that are not separated by a flush may be reordered.
pub struct NvmeDevice {
    pub persisted_content: Vec<u8>,
    // NOTE: we do not use a construct like OrderedWriteLines here.
    // It might happen that when we create permutations, writes to the same block are reordered.
    // But that doesn't matter because when we take partial permutations, no state can appear
    // that could not have appeared otherwise.
    pub unpersisted_content: Vec<Store>,
}

// TODO
const NVME_RANDOM_IMAGES_MAX_AMOUNT: Option<usize> = Some(25);

const NVME_ATOMIC_BLOCK_SIZE_SHIFT: usize = 9;
const NVME_ATOMIC_BLOCK_SIZE: usize = 1 << NVME_ATOMIC_BLOCK_SIZE_SHIFT;
const NVME_ATOMIC_BLOCK_SIZE_MASK: usize = (1 << NVME_ATOMIC_BLOCK_SIZE_SHIFT) - 1;

impl NvmeDevice {
    pub fn new(persisted_content: Vec<u8>) -> Self {
        Self {
            persisted_content,
            unpersisted_content: Vec::new(),
        }
    }

    pub fn generate_nothing_persisted_image(&self, pool: &mut ImagePool) -> CrashHash {
        let (_, hash) = pool.persist(self.persisted_content.as_slice()).unwrap();
        hash
    }

    pub fn generate_everything_persisted_image(&self, pool: &mut ImagePool) -> CrashHash {
        let mut img: Vec<u8> = self.persisted_content.clone();
        for store in self.unpersisted_content.iter() {
            img[store.address_range()].copy_from_slice(store.data.as_slice());
        }
        let (_, hash) = pool.persist(img.as_slice()).unwrap();
        hash
    }

    pub fn generate_random_images(&self, pool: &mut ImagePool, rng: &mut fastrand::Rng) -> HashSet<CrashHash> {
        let mut img: Vec<u8> = vec![0u8; self.persisted_content.len()];
        let mut hashes = HashSet::new();

        if self.unpersisted_content.is_empty() {
            return hashes;
        }

        // TODO use exhaustive if there are less than NVME_RANDOM_IMAGES_MAX_AMOUNT exhaustive
        // images, like vinter does.

        if let Some(amount) = NVME_RANDOM_IMAGES_MAX_AMOUNT {
            let mut indices: Vec<usize> = (0..self.unpersisted_content.len()).collect();
            for _ in 0..amount {
                rng.shuffle(indices.as_mut_slice());
                let partial_index = rng.usize(1..=self.unpersisted_content.len());
                img[..].copy_from_slice(self.persisted_content.as_slice());
                for store in indices[..partial_index].iter().map(|idx| &self.unpersisted_content[*idx]) {
                    img[store.address_range()].copy_from_slice(store.data.as_slice());
                }
                let (_, hash) = pool.persist(img.as_slice()).unwrap();
                hashes.insert(hash);
            }
        } else {
            for indices in (0..self.unpersisted_content.len()).permutations(self.unpersisted_content.len()) {
                img[..].copy_from_slice(self.persisted_content.as_slice());
                for store in indices.into_iter().map(|idx| &self.unpersisted_content[idx]) {
                    img[store.address_range()].copy_from_slice(store.data.as_slice());
                    // create one crash image in every loop execution here, to simulate partial
                    // permutations
                    let (_, hash) = pool.persist(img.as_slice()).unwrap();
                    hashes.insert(hash);
                }
            }
        }
        hashes
    }

    pub fn write(&mut self, address: usize, data: Vec<u8>) {
        if (address & NVME_ATOMIC_BLOCK_SIZE_MASK) != 0 || (data.len() & NVME_ATOMIC_BLOCK_SIZE_MASK) != 0 {
            panic!("unaligned NVMe access: addr={} len={}", address, data.len());
        }
        for offset in (0..data.len()).step_by(NVME_ATOMIC_BLOCK_SIZE) {
            self.unpersisted_content.push(Store {
                address: address + offset,
                data: data[offset..(offset + NVME_ATOMIC_BLOCK_SIZE)].to_vec(),
            });
        }
    }

    pub fn flush(&mut self) {
        for store in self.unpersisted_content.drain(..) {
            self.persisted_content[store.address_range()].copy_from_slice(store.data.as_slice());
        }
    }
}

fn range_overlap<T>(r1: &Range<T>, r2: &Range<T>) -> Range<T>
where
    T: std::cmp::Ord + Copy,
{
    Range {
        start: max(r1.start, r2.start),
        end: min(r1.end, r2.end),
    }
}
