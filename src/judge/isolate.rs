use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    path::PathBuf,
    process::Command,
};

use crate::judge::config::validate_checker;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Default, Debug)]
pub struct Isolate {
    pub box_path: PathBuf,
    pub box_id: u64,
    pub time_limit: f64,
    pub memory_limit: u64,
    pub code: String,
    pub ext: String,
    pub compile_script: String,
    pub run_script: String,
    pub checker: String,
    pub testcases_dir: PathBuf,
    pub initialized: bool,
}

#[derive(Default, PartialEq, Debug)]
pub struct IsolateResult {
    pub status: RunVerdict,
    pub time_usage: f64,
    pub memory_usage: u64,
}

impl Isolate {
    pub fn init(&mut self) -> Result<()> {
        let box_path = Command::new("isolate")
            .arg("--cg")
            .arg(format!("--box-id={}", self.box_id))
            .arg("--init")
            .output()?;

        anyhow::ensure!(
            box_path.status.success(),
            "Failed to init isolate box {}: {}",
            self.box_id,
            String::from_utf8_lossy(&box_path.stderr)
        );

        self.box_path = PathBuf::from(String::from_utf8(box_path.stdout)?.trim()).join("box");
        self.initialized = true;

        fs::write(
            self.box_path.join(format!("source.{}", self.ext)),
            &self.code,
        )?;

        Ok(())
    }

    pub fn compile(&mut self) -> Result<IsolateResult> {
        let compile_box_id = self.box_id + 500;
        let box_path = Command::new("isolate")
            .arg("--cg")
            .arg(format!("--box-id={compile_box_id}"))
            .arg("--init")
            .output()?;

        anyhow::ensure!(
            box_path.status.success(),
            "Failed to init compile isolate box {compile_box_id}: {}",
            String::from_utf8_lossy(&box_path.stderr)
        );

        let compile_box = PathBuf::from(String::from_utf8(box_path.stdout)?.trim()).join("box");
        let source_file = format!("source.{}", self.ext);

        let res = (|| -> Result<RunVerdict> {
            fs::write(compile_box.join(&source_file), &self.code)?;

            let compile_script = self
                .compile_script
                .replace("{source_file}", &source_file)
                .replace("{output}", "source");

            let output = Command::new("isolate")
                .arg("--cg")
                .arg(format!("--box-id={compile_box_id}"))
                .arg("--time=10")
                .arg("--wall-time=15")
                .arg("--extra-time=1")
                .arg("--cg-mem=1048576")
                .arg("--processes=128")
                .arg("--env=PATH=/usr/bin:/bin")
                .arg("--run")
                .arg("--")
                .args(compile_script.split(' '))
                .output()?;

            if output.status.success() {
                let compiled_bin = compile_box.join("source");
                if compiled_bin.exists() {
                    fs::copy(&compiled_bin, self.box_path.join("source"))?;
                }
                Ok(RunVerdict::VerdictOK)
            } else {
                Ok(RunVerdict::CompilationError)
            }
        })();

        let _ = Command::new("isolate")
            .arg("--cg")
            .arg(format!("--box-id={compile_box_id}"))
            .arg("--cleanup")
            .output();

        Ok(IsolateResult {
            status: res?,
            ..Default::default()
        })
    }

    pub fn check(&mut self, test_index: u64) -> Result<bool> {
        validate_checker(&self.checker).map_err(anyhow::Error::msg)?;

        let output = Command::new("timeout")
            .arg("10")
            .arg(format!("checker/{}", self.checker))
            .arg(self.testcases_dir.join(format!("{}.in", test_index)))
            .arg(self.box_path.join("out.out"))
            .arg(self.testcases_dir.join(format!("{}.sol", test_index)))
            .output()?;

        anyhow::ensure!(
            output.status.code() != Some(124),
            "Checker execution timed out after 10 seconds"
        );

        let is_correct = if output.stdout.starts_with(b"Correct\n100") {
            true
        } else if output.stdout.starts_with(b"Incorrect") {
            false
        } else {
            output.status.success() && output.stdout.is_empty()
        };

        Ok(is_correct)
    }

    pub fn meta_path(&self) -> PathBuf {
        self.box_path
            .parent()
            .unwrap_or(&self.box_path)
            .join("meta.txt")
    }

    pub fn run(&mut self, test_index: u64) -> Result<IsolateResult> {
        let run_script = self.run_script.replace("{source}", "source");
        let input_file = File::open(self.testcases_dir.join(format!("{}.in", test_index)))?;

        Command::new("isolate")
            .arg("--cg")
            .arg(format!("--box-id={}", self.box_id))
            .arg(format!("--time={}", self.time_limit))
            .arg(format!("--wall-time={}", (self.time_limit + 5.0)))
            .arg(format!("--extra-time={}", (self.time_limit + 1.0)))
            .arg(format!("--cg-mem={}", self.memory_limit))
            .arg(format!("--meta={}", self.meta_path().display()))
            .stdin(input_file)
            .arg("--stdout=out.out")
            .arg("--processes=128")
            .arg("--run")
            .arg("--")
            .args(run_script.split(' '))
            .output()?;

        let result = self.get_result()?;

        Ok(result)
    }

    pub fn get_result(&self) -> Result<IsolateResult> {
        let mut result: IsolateResult = Default::default();
        let mut memory_limit_exceeded = false;

        let meta = fs::read_to_string(self.meta_path())?;

        for meta_line in meta.lines() {
            if let Some((key, val)) = meta_line.split_once(':') {
                match key {
                    "status" => {
                        result.status = match val {
                            "RE" => RunVerdict::VerdictRE,
                            "SG" => RunVerdict::VerdictSG,
                            "TO" => RunVerdict::VerdictTLE,
                            "XX" => RunVerdict::VerdictXX,
                            _ => RunVerdict::VerdictSG,
                        };
                    }
                    "time" => result.time_usage = val.parse()?,
                    "cg-mem" => result.memory_usage = val.parse()?,
                    "cg-oom-killed" => memory_limit_exceeded = val.trim() == "1",
                    _ => (),
                }
            }
        }
        if memory_limit_exceeded || result.memory_usage >= self.memory_limit {
            result.status = RunVerdict::VerdictMLE;
        }

        Ok(result)
    }

    pub fn cleanup(&mut self) -> Result<()> {
        if self.initialized {
            self.initialized = false;
            Command::new("isolate")
                .arg("--cg")
                .arg(format!("--box-id={}", self.box_id))
                .arg("--cleanup")
                .output()?;
        }

        Ok(())
    }
}

impl Drop for Isolate {
    fn drop(&mut self) {
        if let Err(err) = self.cleanup() {
            tracing::warn!(box_id = self.box_id, error = %err, "Failed to cleanup isolate sandbox");
        }
    }
}

/// What `judge::runner`'s scoring logic (run_normal/run_subtask) actually
/// needs from a sandbox. Lets that logic be tested with a fake instead of
/// requiring a real isolate CLI + cgroups on the test host.
pub trait Sandbox {
    fn run(&mut self, test_index: u64) -> Result<IsolateResult>;
    fn check(&mut self, test_index: u64) -> Result<bool>;
}

impl Sandbox for Isolate {
    fn run(&mut self, test_index: u64) -> Result<IsolateResult> {
        Isolate::run(self, test_index)
    }

    fn check(&mut self, test_index: u64) -> Result<bool> {
        Isolate::check(self, test_index)
    }
}
