use std::fs::File;
use std::io::Write;
use std::collections::HashSet;

use anyhow::{Context, Result};

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct CrashHash(blake3::Hash);                                                                                   
                                                                                                                      
impl serde::Serialize for CrashHash {                                                                                 
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>                                                  
    where                                                                                                             
        S: serde::Serializer                                                                                          
    {                                                                                                                 
        serializer.serialize_str(&self.0.to_hex())                                                                    
    }                                                                                                                 
}

pub struct ImagePool {
    crash_dir: String,
    size: usize,
    size_max: usize,
    hashes: HashSet<CrashHash>,
}

impl ImagePool {
    pub fn new(work_dir: &String) -> Result<Self> {
        let crash_dir = format!("{}/crash_images", work_dir);
        std::fs::create_dir(crash_dir.as_str()).context("could not create crash images directory")?;
        Ok(Self {
            crash_dir,
            size: 0,
            size_max: usize::MAX,
            hashes: HashSet::new(),
        })
    }

    pub fn with_limit(work_dir: &String, limit: usize) -> Result<Self> {
        let mut pool = Self::new(work_dir)?;
        pool.size_max = limit;
        Ok(pool)
    }

    pub fn persist(&mut self, data: &[u8]) -> Result<(bool, CrashHash)> {
        let hash = CrashHash(blake3::hash(data));
        if self.hashes.insert(hash.clone()) {
            // first time encountering this hash
            self.size += data.len();
            if self.size > self.size_max {
                panic!("image pool size limit exceeded");
            }
            let path = format!("{}/{}.raw", self.crash_dir, hash.0.to_hex());
            let mut file = File::create(path.as_str()).with_context(|| format!("could not open {}", path))?;
            file.write_all(data).context("could not dump image")?;
            Ok((true, hash))
        } else {
            Ok((false, hash))
        }
    }
}
