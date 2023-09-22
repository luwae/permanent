use std::io::Write;
use crossbeam_channel::{Receiver, select};
use std::collections::{VecDeque};

use permanent_common::trace::{PmemEvent, NvmeEvent, TraceEntry, TraceWriter};

#[derive(Debug)]
pub enum TraceMessage {
    Pmem(PmemEvent),
    NvmeFlush,
    PciNvmeBlkRead {
        req: u64,
        offset: u64,
    },
    PciNvmeBlkWrite {
        req: u64,
        offset: u64,
    },
    DmaBlkIo {
        req: u64,
        dbs: u64,
    },
    DmaBlkRead {
        dbs: u64,
        offset: i64,
        length: i64,
    },
    DmaBlkWrite {
        dbs: u64,
        offset: i64,
        length: i64,
        data: Vec<u8>,
    },
    PciNvmeEnqueueReqCompletion {
        req: u64,
    },
    Checkpoint {
        value: u8,
    },
}

#[derive(Debug)]
struct NvmeConsolidateInfo {
    id: usize,
    req: u64,
    dbs: u64,
}

struct TraceQueue<W: Write> {
    tail_id: usize,
    queue: VecDeque<TraceEntry>,
    consolidate: VecDeque<NvmeConsolidateInfo>, // entries are ordered by id
    trace_out: TraceWriter<W>
}

fn write_entry<W: Write>(entry: TraceEntry, dst: &mut W) {
    if cfg!(permanent_trace_debug = "entry") {
        println!("{:?}", entry);
    } else {
        entry.serialize_into(dst).expect("failed encoding trace entry");
    }
}

impl<W: Write> TraceQueue<W> {
    fn queue_index(&self, id: usize) -> usize {
        id - (self.tail_id - self.queue.len())
    }

    fn insert_complete(&mut self, entry: TraceEntry) {
        if self.queue.len() == 0 {
            write_entry(entry, &mut self.trace_out);
        } else {
            self.queue.push_back(entry);
        }
        self.tail_id += 1;
    }

    fn insert_incomplete(&mut self, entry: TraceEntry, req: u64) {
        self.queue.push_back(entry);
        self.consolidate.push_back(NvmeConsolidateInfo { id: self.tail_id, req, dbs: 0 });
        self.tail_id += 1;
    }

    fn insert(&mut self, msg: TraceMessage) {
        let id64 = self.tail_id as u64;
        match msg {
            TraceMessage::Checkpoint { value } => {
                self.insert_complete(TraceEntry::Checkpoint { id: id64, value });
            },
            TraceMessage::Pmem(event) => {
                self.insert_complete(TraceEntry::Pmem { id: id64, event });
            },
            TraceMessage::NvmeFlush => {
                self.insert_complete(TraceEntry::Nvme { id: id64, event: NvmeEvent::Flush });
            },
            TraceMessage::PciNvmeBlkRead { req, offset } => {
                let entry = TraceEntry::Nvme { id: id64, event: NvmeEvent::Read { offset, length: 0 }};
                self.insert_incomplete(entry, req);
            },
            TraceMessage::PciNvmeBlkWrite { req, offset } => {
                let entry = TraceEntry::Nvme { id: id64, event: NvmeEvent::Write { offset, length: 0, data: Vec::new() }};
                self.insert_incomplete(entry, req);
            },
            TraceMessage::DmaBlkIo { req, dbs } => {
                // we do not panic on unfound req, because revin doesn't do this either.
                // might have a problem here if req and dbs pointers get reused. Revin fills this in reverse order, we don't.
                //   TODO find out if revin has a reason for this besides performance
                if let Some(info) = self.consolidate.iter_mut().find(|x| x.req == req) {
                    (*info).dbs = dbs;
                }
            },
            TraceMessage::DmaBlkRead { dbs, offset: _, length } => {
                if let Some(info) = self.consolidate.iter().find(|x| x.dbs == dbs) {
                    match self.queue.get_mut(self.queue_index(info.id)).unwrap() {
                        TraceEntry::Nvme { id: _, event: NvmeEvent::Read { offset: _, length: length_ref } } => {
                            *length_ref = length.try_into().unwrap();
                        },
                        other => panic!("TraceEntry should be NvmeRead but is {:?}", other),
                    }
                }
            },
            TraceMessage::DmaBlkWrite { dbs, offset: _, length, data } => {
                if let Some(info) = self.consolidate.iter().find(|x| x.dbs == dbs) {
                    match self.queue.get_mut(self.queue_index(info.id)).unwrap() {
                        TraceEntry::Nvme { id: _, event: NvmeEvent::Write { offset: _, length: length_ref, data: data_ref } } => {
                            *length_ref = length.try_into().unwrap();
                            *data_ref = data;
                        },
                        other => panic!("TraceEntry should be NvmeWrite but is {:?}", other),
                    }
                }
            },
            TraceMessage::PciNvmeEnqueueReqCompletion { req } => {
                if let Some(i) = self.consolidate.iter().position(|x| x.req == req) {
                    self.consolidate.remove(i); // O(n) worst case, but we don't use swap_remove because we want to preserve id order
                                                // (most often we remove the front element anyways)
                    if i == 0 { // oldest entry has been freed, so we can write something out
                        let drain_until = match self.consolidate.get(0) {
                            Some(cons_entry) => self.queue_index(cons_entry.id),
                            None => self.queue.len(), // drain everything; we don't have remaining entries.
                        };
                        for entry in self.queue.drain(0..drain_until) {
                            write_entry(entry, &mut self.trace_out);
                        }
                    }
                }
            }
        }
    }
}

pub fn writer_main<W: Write>(trace_recv: Receiver<TraceMessage>, done_recv: Receiver<()>, trace_out: TraceWriter<W>) {
    let mut q = TraceQueue { tail_id: 0, queue: VecDeque::new(), consolidate: VecDeque::new(), trace_out };
    loop {
        select! {
            recv(trace_recv) -> msg => {
                let msg = msg.unwrap();
                if cfg!(permanent_trace_debug = "message") {
                    println!("{:?}", msg);
                } else {
                    q.insert(msg);
                }
            }
            recv(done_recv) -> _ => {
                println!("permanent_plugin: writer thread shut down");
                if q.queue.len() > 0 {
                    panic!("writer thread quit with queue non-empty");
                }
                q.trace_out.flush().unwrap();
                break;
            }
        }
    }
}
