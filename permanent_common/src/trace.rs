use std::io::{BufRead, Read, Write};

use serde::{Serialize, Deserialize};
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
pub enum PmemEvent {
    Read {
        address: u64,
        size: u64,
        content: Vec<u8>,
    },
    Write {
        address: u64,
        size: u64,
        content: Vec<u8>,
        non_temporal: bool,
    },
    Fence,
    Clflush { address: u64 },
    Clflushopt { address: u64 },
    Clwb { address: u64 },
    Wbinvd,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NvmeEvent {
    Read {
        offset: u64,
        length: u64,
    },
    Write {
        offset: u64,
        length: u64,
        data: Vec<u8>,
    },
    Flush,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TraceEntry {
    Pmem { id: u64, event: PmemEvent },
    Nvme { id: u64, event: NvmeEvent },
    Checkpoint { id: u64, value: u8 },
}

impl TraceEntry {
    pub fn deserialize_from<R: Read>(src: &mut R) -> bincode::Result<TraceEntry> {
        bincode::deserialize_from(src)
    }

    pub fn serialize_into<W: Write>(&self, dst: &mut W) -> bincode::Result<()> {
        bincode::serialize_into(dst, self)
    }
}

pub struct BinTraceIterator<R: Read> {
    file: R,
}

fn is_eof(err: &bincode::ErrorKind) -> bool {
    match err {
        bincode::ErrorKind::Io(io_err) => match io_err.kind() {
            std::io::ErrorKind::UnexpectedEof => true,
            _ => false,
        },
        _ => false,
    }
}

impl<R: Read> Iterator for BinTraceIterator<R> {
    type Item = Result<TraceEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        match TraceEntry::deserialize_from(&mut self.file) {
            Ok(e) => Some(Ok(e)),
            Err(e) if is_eof(&*e) => None,
            Err(e) => Some(Err(e.into())),
        }
    }
}

pub type TraceWriter<W> = snap::write::FrameEncoder<W>;

/// Create a trace writer with compression.
pub fn new_trace_writer_bin<W: Write>(file: W) -> TraceWriter<W> {
    snap::write::FrameEncoder::new(file)
}

/// Parse a binary trace file.
pub fn parse_trace_file_bin<R: BufRead>(file: R) -> BinTraceIterator<snap::read::FrameDecoder<R>> {
    BinTraceIterator {
        file: snap::read::FrameDecoder::new(file),
    }
}
