use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum RunVerdict {
    #[serde(rename = "Compilation Error")]
    CompilationError,
    #[serde(rename = "Accepted")]
    VerdictOK,
    #[serde(rename = "Time Limit Exceeded")]
    VerdictTLE,
    #[serde(rename = "Memory Limit Exceeded")]
    VerdictMLE,
    #[serde(rename = "Runtime Error")]
    VerdictRE,
    #[serde(rename = "Internal Error")]
    VerdictXX,
    #[serde(rename = "Signal Error")]
    VerdictSG,
}

impl Default for RunVerdict {
    fn default() -> Self {
        Self::VerdictOK
    }
}

#[derive(Default, PartialEq, Debug)]
pub struct IsolateResult {
    pub status: RunVerdict,
    pub time_usage: f64,
    pub memory_usage: u64,
}

#[derive(Debug, Clone)]
pub struct Limits {
    pub time: f64,
    pub wall_time: f64,
    pub extra_time: f64,
    pub memory_kb: u64,
    pub processes: u32,
    pub fsize_kb: u64,
}

impl Limits {
    pub fn compile() -> Self {
        Self {
            time: 10.0,
            wall_time: 15.0,
            extra_time: 1.0,
            memory_kb: 1_048_576,
            processes: 128,
            fsize_kb: 262_144,
        }
    }

    pub fn run(time: f64, memory_kb: u64) -> Self {
        Self {
            time,
            wall_time: time + 5.0,
            extra_time: time + 1.0,
            memory_kb,
            processes: 128,
            fsize_kb: 65_536,
        }
    }
}

#[derive(Debug)]
pub struct IsolateBox {
    id: u64,
    path: PathBuf,
    meta_path: PathBuf,
}

impl IsolateBox {
    pub fn open(id: u64) -> Result<Self> {
        let output = Self::command(id).arg("--init").output()?;

        anyhow::ensure!(
            output.status.success(),
            "Failed to init isolate box {id}: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        Ok(Self {
            id,
            path: PathBuf::from(String::from_utf8(output.stdout)?.trim()).join("box"),
            meta_path: std::env::temp_dir().join(format!("isolate_meta_{id}.txt")),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn write(&self, name: &str, bytes: impl AsRef<[u8]>) -> Result<()> {
        fs::write(self.path.join(name), bytes)?;
        Ok(())
    }

    pub fn exec(
        &self,
        cmd: &[String],
        limits: &Limits,
        stdin: Option<File>,
        stdout: Option<&str>,
        env: &[&str],
    ) -> Result<IsolateResult> {
        let mut command = Self::command(self.id);
        command
            .arg(format!("--time={}", limits.time))
            .arg(format!("--wall-time={}", limits.wall_time))
            .arg(format!("--extra-time={}", limits.extra_time))
            .arg(format!("--cg-mem={}", limits.memory_kb))
            .arg(format!("--processes={}", limits.processes))
            .arg(format!("--fsize={}", limits.fsize_kb))
            .arg(format!("--meta={}", self.meta_path.display()));
        if let Some(stdin) = stdin {
            command.stdin(stdin);
        }
        if let Some(stdout) = stdout {
            command.arg(format!("--stdout={stdout}"));
        }
        for var in env {
            command.arg(format!("--env={var}"));
        }
        command.arg("--run").arg("--").args(cmd).output()?;

        parse_meta(&fs::read_to_string(&self.meta_path)?, limits.memory_kb)
    }

    fn command(id: u64) -> Command {
        let mut command = Command::new("isolate");
        command.arg("--cg").arg(format!("--box-id={id}"));
        command
    }
}

impl Drop for IsolateBox {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.meta_path);
        if let Err(err) = Self::command(self.id).arg("--cleanup").output() {
            tracing::warn!(box_id = self.id, error = %err, "Failed to cleanup isolate sandbox");
        }
    }
}

pub fn parse_meta(meta: &str, memory_limit_kb: u64) -> Result<IsolateResult> {
    let mut result = IsolateResult::default();
    let mut memory_limit_exceeded = false;

    for meta_line in meta.lines() {
        if let Some((key, val)) = meta_line.split_once(':') {
            match key {
                "status" => {
                    result.status = match val.trim() {
                        "RE" => RunVerdict::VerdictRE,
                        "SG" => RunVerdict::VerdictSG,
                        "TO" => RunVerdict::VerdictTLE,
                        "XX" => RunVerdict::VerdictXX,
                        _ => RunVerdict::VerdictSG,
                    };
                }
                "time" => result.time_usage = val.trim().parse()?,
                "cg-mem" => result.memory_usage = val.trim().parse()?,
                "cg-oom-killed" => memory_limit_exceeded = val.trim() == "1",
                _ => (),
            }
        }
    }
    if memory_limit_exceeded || result.memory_usage >= memory_limit_kb {
        result.status = RunVerdict::VerdictMLE;
    }

    Ok(result)
}
