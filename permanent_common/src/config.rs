use enumset::{EnumSet, EnumSetType};
use serde::Deserialize;

#[derive(Debug, EnumSetType)]
pub enum TraceOption {
    PmemRead,
    PmemWrite,
    PmemFence,
    PmemFlush,

    NvmeRead,
    NvmeWrite,
    NvmeFlush,

    Checkpoint
}

impl TraceOption {
    pub fn to_qemu_str(&self) -> &'static str {
        match self {
            TraceOption::PmemRead => "pmem_read",
            TraceOption::PmemWrite => "pmem_write",
            TraceOption::PmemFence => "pmem_fence",
            TraceOption::PmemFlush => "pmem_flush",

            TraceOption::NvmeRead => "nvme_read",
            TraceOption::NvmeWrite => "nvme_write",
            TraceOption::NvmeFlush => "nvme_flush",

            TraceOption::Checkpoint => "checkpoint",
        }
    }

    pub fn from_qemu_str(s: &str) -> Result<Self, ()> {
        match s {
            "pmem_read" => Ok(TraceOption::PmemRead),
            "pmem_write" => Ok(TraceOption::PmemWrite),
            "pmem_fence" => Ok(TraceOption::PmemFence),
            "pmem_flush" => Ok(TraceOption::PmemFlush),

            "nvme_read" => Ok(TraceOption::NvmeRead),
            "nvme_write" => Ok(TraceOption::NvmeWrite),
            "nvme_flush" => Ok(TraceOption::NvmeFlush),

            "checkpoint" => Ok(TraceOption::Checkpoint),

            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct TcgPluginConfig {
    pub pmem_start: u64,
    pub pmem_len: u64,
    pub pmem_base_image_path: Option<String>,
    pub trace_what: EnumSet<TraceOption>,
    pub out_trace_file: String,
}

impl TcgPluginConfig {
    pub fn to_qemu_plugin_arg_string(&self, plugin_path: &str) -> String {
        // TODO there is probably a nicer way, but intersperse() is unstable
        let maybe_trace_what_string = if self.trace_what.is_empty() {
            None
        } else  {
            let mut trace_what_string = String::new();
            for opt in self.trace_what.iter().map(|o| o.to_qemu_str()) {
                trace_what_string.push_str(opt);
                trace_what_string.push('/');
            }
            trace_what_string.pop(); // remove last separator
            Some(trace_what_string)
        };

        let mut s = format!("{},pmem_start={},pmem_len={}",
                plugin_path,
                self.pmem_start,
                self.pmem_len,
        );
        if let Some(path) = &self.pmem_base_image_path {
            s.push_str(format!(",pmem_base_image_path={}", path).as_str());
        }
        if let Some(trace_what_string) = maybe_trace_what_string {
            s.push_str(format!(",trace_what={}", trace_what_string).as_str());
        }
        s.push_str(format!(",out_trace_file={}", self.out_trace_file).as_str());
        s
    }
}

/// vm.yaml files
#[derive(Clone, Debug, Deserialize)]
pub struct VmConfig {
    pub fs_type: String,
    pub pmem_start: Option<u64>, // only used for pmem/hybrid; yaml files can simply leave it out
    pub pmem_len: Option<u64>,
    pub qemu_path: String,
    pub kernel_path: String,
    pub initrd_path: String,
    pub qemu_args: Vec<String>,
    pub trace_cmd_prefix: String,
    pub dump_cmd_prefix: String,
    pub recovery_cmd: String,
}

impl VmConfig {
    pub fn have_pmem_nvme(&self) -> (bool, bool) {
        match self.fs_type.as_str() {
            "pmem" => (true, false),
            "nvme" => (false, true),
            "hybrid" => (true, true),
            _ => panic!("invalid fs_type in vm config"),
        }
    }
}

/// test.yaml files
#[derive(Clone, Debug, Deserialize)]
pub struct TestConfig {
    pub trace_cmd_suffix: String,
    pub checkpoint_range: (u8, u8),
    pub dump_cmd_suffix: String,
}

#[derive(Clone)]
pub enum TraceType {
    /// execute test case. Trace all writes/fences/flushes/checkpoints
    Analyse,
    /// do recovery trace. Trace all reads/checkpoints
    PostSuccess,
    /// dump file system and verify integrity. Trace all checkpoints
    PostFailure { pmem_hash: Option<String>, nvme_hash: Option<String> },
}

/// configuration for a single tracing operation
#[derive(Clone)]
pub struct TraceConfig {
    pub trace_type: TraceType,
    dir: String,
}

impl TraceConfig {
    pub fn new(work_dir: &String, trace_type: TraceType) -> Self {
        let prefix = match &trace_type {
            TraceType::Analyse => "analyse".to_string(),
            TraceType::PostSuccess => "post_success".to_string(),
            TraceType::PostFailure { pmem_hash, nvme_hash } => {
                let mut s = "post".to_string();
                if let Some(hash) = pmem_hash {
                    s.push('_');
                    s.push_str(hash);
                }
                if let Some(hash) = nvme_hash {
                    s.push('_');
                    s.push_str(hash);
                }
                s
            }
        };
        Self {
            trace_type,
            dir: format!("{}/{}", work_dir, prefix),
        }
    }

    pub fn trace_dir(&self) -> String {
        self.dir.clone()
    }

    pub fn pipe_path(&self) -> String {
        format!("{}/pipe", self.dir)
    }

    pub fn trace_path(&self) -> String {
        format!("{}/trace.bin", self.dir)
    }

    pub fn pmem_image_path(&self) -> String {
        format!("{}/pmem.raw", self.dir)
    }

    pub fn nvme_image_path(&self) -> String {
        format!("{}/nvme.raw", self.dir)
    }
    
    pub fn log_path(&self) -> String {
        format!("{}/log", self.dir)
    }
    
    pub fn io_log_path(&self) -> String {
        format!("{}/io_log", self.dir)
    }
}
