use std::env;
use std::io::BufReader;
use std::fs::File;

use permanent_common::trace::parse_trace_file_bin;
use permanent_common::trace::{PmemEvent, NvmeEvent, TraceEntry};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        println!("usage: {} <FILE> [--nodata]", args[0]);
        return;
    }
    
    let file = File::open(args[1].as_str()).unwrap();
    let nodata = args.len() == 3 && args[2] == "--nodata";
    for item in parse_trace_file_bin(BufReader::new(file)) {
        let mut item = item.unwrap();
        if nodata {
            remove_data(&mut item);
        }
        println!("{:?}", item);
    }
}

fn remove_data(item: &mut TraceEntry) {
    match item {
        //TraceEntry::Pmem { id: _, event } => {
        //    match event {
        //        PmemEvent::Read { address: _, size: _, content } => { content.clear(); },
        //        PmemEvent::Write { address: _, size: _, content, non_temporal: _ } => { content.clear(); },
        //        _ => { },
        //    }
        //},
        TraceEntry::Nvme { id: _, event } => {
            match event {
                NvmeEvent::Write { offset: _, length: _, data } => { data.clear(); },
                _ => { },
            }
        },
        _ => { },
    }
}
