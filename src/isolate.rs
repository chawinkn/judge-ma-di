use std::{ fs, path::PathBuf, str::from_utf8 };
use tokio::process::Command;
use std::env;
use anyhow::Result;

#[derive(Debug, PartialEq)]
pub enum RunVerdict {
    CompilationError,
    VerdictOK,
    VerdictTLE,
    VerdictMLE,
    VerdictRE,
    VerdictXX,
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
    pub input_path: PathBuf,
    pub task_id: String,
    pub ext: String,
    pub compile_script: String,
    pub run_script: String,
    pub checker: String,
}

#[derive(Default, PartialEq, Debug)]
pub struct IsolateResult {
    pub status: RunVerdict,
    pub time_usage: f64,
    pub memory_usage: u64,
}

impl Isolate {
    pub async fn init(&mut self) -> Result<()> {
        let box_path = Command::new("isolate")
            .arg("--cg")
            .arg(format!("--box-id={}", self.box_id))
            .arg("--init")
            .output().await?;

        let box_path = String::from_utf8(box_path.stdout)?;
        self.box_path = PathBuf::from(box_path.trim()).join("box");

        let current_dir = env::current_dir()?;
        let source_path = current_dir.join("temp").join(&self.input_path);
        let destination_path = self.box_path.join(format!("source.{}", self.ext));
        fs::copy(&source_path, &destination_path)?;

        let input_path = current_dir.join("tasks").join(&self.task_id).join("testcases");

        for entry in fs::read_dir(input_path)? {
            let entry = entry?;
            let path = entry.path();

            let destination_path = self.box_path.join(path.file_name().unwrap_or_default());
            fs::copy(&path, &destination_path)?;
        }

        Ok(())
    }

    pub async fn compile(&mut self) -> Result<IsolateResult> {
        let mut compile_script = self.compile_script.replace(
            "{source_file}",
            &format!("{}/source.{}", self.box_path.display(), self.ext)
        );
        if self.ext != "py" {
            compile_script = compile_script.replace(
                "{output}",
                &format!("{}/source", self.box_path.display())
            );
        }

        let split: Vec<&str> = compile_script.split(' ').collect();
        let output = Command::new(split[0])
            .args(&split[1..])
            .output().await?;

        let mut result: IsolateResult = Default::default();
        if !output.status.success() {
            result.status = RunVerdict::CompilationError;
        }

        Ok(result)
    }

    pub async fn check(&mut self, test_index: i32) -> Result<bool> {
        let current_dir = env::current_dir()?;
        let checker_dir = current_dir.join("checker");

        let result = Command::new(format!("{}/{}", checker_dir.display(), self.checker))
            .arg(format!("{}/{}.in", self.box_path.display(), test_index))
            .arg(format!("{}/out.out", self.box_path.display()))
            .arg(format!("{}/{}.sol", self.box_path.display(), test_index))
            .output().await?;

        let stdout = from_utf8(&result.stdout).unwrap().to_string();

        Ok(stdout == "Correct\n100\n".to_string())
    }

    pub async fn run(&mut self, test_index: i32) -> Result<IsolateResult> {
        let run_script = self.run_script.replace("{source}", "source");
        let split: Vec<&str> = run_script.split(' ').collect();

        Command::new("isolate")
            .arg("--cg")
            .arg(format!("--box-id={}", self.box_id))
            .arg(format!("--time={}", self.time_limit.to_string()))
            .arg(format!("--wall-time={}", (self.time_limit + 5.0).to_string()))
            .arg(format!("--extra-time={}", (self.time_limit + 1.0).to_string()))
            .arg(format!("--cg-mem={}", self.memory_limit))
            .arg(format!("--meta={}/meta.txt", self.box_path.display()))
            .arg(format!("--stdin={}.in", test_index))
            .arg("--stdout=out.out")
            // .arg("--processes=128")
            .arg("--run")
            .arg("--")
            .args(split)
            .output().await?;

        let result = self.get_result().await?;

        Ok(result)
    }

    pub async fn get_result(&self) -> Result<IsolateResult> {
        let mut result: IsolateResult = Default::default();
        let mut memory_limit_exceeded = false;

        let meta = fs::read_to_string(format!("{}/meta.txt", self.box_path.display()))?;

        for meta_line in meta.lines() {
            let args: Vec<&str> = meta_line.split(":").collect();
            if args.len() >= 2 {
                match args[0] {
                    "status" => {
                        result.status = match args[1] {
                            "RE" => RunVerdict::VerdictRE,
                            "SG" => RunVerdict::VerdictSG,
                            "TO" => RunVerdict::VerdictTLE,
                            "XX" => RunVerdict::VerdictXX,
                            _ => RunVerdict::VerdictSG,
                        };
                    }
                    "time" => {
                        result.time_usage = args[1].parse()?;
                    }
                    "cg-mem" => {
                        result.memory_usage = args[1].parse()?;
                    }
                    "cg-oom-killed" => {
                        memory_limit_exceeded = args[1].trim() == "1";
                    }
                    _ => (),
                }
            }
        }
        if memory_limit_exceeded || result.memory_usage >= self.memory_limit {
            result.status = RunVerdict::VerdictMLE;
        }

        Ok(result)
    }

    pub async fn cleanup(&mut self) -> Result<()> {
        Command::new("isolate")
            .arg("--cg")
            .arg(format!("--box-id={}", self.box_id))
            .arg("--cleanup")
            .output().await?;

        Ok(())
    }
}
