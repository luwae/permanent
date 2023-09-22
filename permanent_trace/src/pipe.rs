use std::os::unix::fs::OpenOptionsExt;
use std::io;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::io::BufReader;
use std::io::BufWriter;
use std::fs::File;
use std::io::BufRead;
use std::io::Write;
extern crate libc;

pub struct Pipe {
    pub reader: BufReader<File>,
    pub writer: BufWriter<File>,
    logger: BufWriter<File>,
}

impl Pipe {
    pub fn open(path: &String, logger: BufWriter<File>) -> io::Result<Pipe> {
        let pipe_out = fs::OpenOptions::new()
                        .read(true)
                        .custom_flags(libc::O_NONBLOCK)
                        .open(&format!("{}.out", path))?;
        let pipe_in = fs::OpenOptions::new()
                        .write(true)
                        .append(true)
                        .open(&format!("{}.in", path))?;
        let pipe = Pipe {
            reader: BufReader::new(pipe_out),
            writer: BufWriter::new(pipe_in),
            logger
        };

        Ok(pipe)
    }

    pub fn make(path: &String) -> io::Result<()> {
        if Path::new(&format!("{}.in", path)).exists() {
            fs::remove_file(format!("{}.in", path))?;
        }
        if Path::new(&format!("{}.out", path)).exists() {
            fs::remove_file(format!("{}.out", path))?;
        }

        let mut handler = Command::new("mkfifo")
            .arg(format!("{}.in", path))
            .arg(format!("{}.out", path))
            .spawn()?;
        handler.wait()?;

        Ok(())
    }

    pub fn wait_for_any(&mut self, variants: &[&[u8]]) -> Result<usize, io::Error> {
        let mut wait_iters = 200;
        let mut buf: Vec<u8> = Vec::new();

        loop {
            let result = self.reader.read_until(b'\n', &mut buf);
            if let Err(e) = result {
                if e.kind() == io::ErrorKind::WouldBlock {
                    if wait_iters <= 0 {
                        self.logger.write_fmt(format_args!("== Pipe broken")).unwrap();
                        return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Pipe locked"));
                    }
                    wait_iters -= 1;
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    continue;
                } else {
                    return Err(e);
                }
            } else {
                wait_iters = 200;
            }

            let n = result.unwrap();
            if n == 0 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Unexpected EOF"));
            }

            self.logger.write_all(buf.as_slice()).unwrap();
            self.logger.flush().unwrap(); // we might want to see output immediately to debug

            for (i, variant) in variants.iter().enumerate() {
                if buf.windows(variant.len()).any(|win| win == *variant) {
                    return Ok(i)
                }
            }
            buf.clear();
        }
    }

    pub fn wait_for(&mut self, bytes: &[u8]) -> Result<(), io::Error> {
        let single_variant = [bytes];
        match self.wait_for_any(&single_variant) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn send(&mut self, text: &str) -> Result<(), io::Error> {
        self.writer.write_fmt(format_args!("{}", text))?;
        self.writer.flush()?;
        Ok(())
    }
}
