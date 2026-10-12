use anyhow::Result;
use std::{
    fs::{self, File},
    path::Path,
    process::Command,
};

use crate::judge::isolate::{IsolateBox, IsolateResult, Limits, RunVerdict};
use crate::judge::languages::Language;
use crate::judge::task::{validate_checker, Task};

pub trait Sandbox {
    /// Returns false on a compilation error. Called first, exactly once.
    fn compile(&mut self) -> Result<bool>;
    fn run(&mut self, test_index: u64) -> Result<IsolateResult>;
    fn check(&mut self, test_index: u64) -> Result<bool>;
}

pub struct IsolateSandbox<'a> {
    box_id: u64,
    run_box: Option<IsolateBox>,
    language: &'static dyn Language,
    task: &'a Task,
    code: String,
}

impl<'a> IsolateSandbox<'a> {
    pub fn new(box_id: u64, task: &'a Task, language: &'static dyn Language, code: String) -> Self {
        Self {
            box_id,
            run_box: None,
            language,
            task,
            code,
        }
    }

    fn run_box(&self) -> Result<&IsolateBox> {
        self.run_box
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("sandbox used before a successful compile()"))
    }
}

impl Sandbox for IsolateSandbox<'_> {
    fn compile(&mut self) -> Result<bool> {
        let run_box = IsolateBox::open(self.box_id)?;
        run_box.write(&self.language.source_filename(), &self.code)?;

        if let Some(compile_command) = self.language.compile_command() {
            let compile_box = IsolateBox::open(self.box_id + 500)?;
            compile_box.write(&self.language.source_filename(), &self.code)?;

            let result = compile_box.exec(
                &compile_command,
                &Limits::compile(),
                None,
                None,
                &["PATH=/usr/bin:/bin"],
            )?;
            if result.status != RunVerdict::VerdictOK {
                return Ok(false);
            }

            if let Some(artifact) = self.language.compiled_artifact() {
                let compiled_bin = compile_box.path().join(artifact);
                if compiled_bin.exists() {
                    fs::copy(&compiled_bin, run_box.path().join(artifact))?;
                }
            }
        }

        self.run_box = Some(run_box);
        Ok(true)
    }

    fn run(&mut self, test_index: u64) -> Result<IsolateResult> {
        let manifest = self.task.manifest();
        let limits = Limits::run(manifest.time_limit, manifest.memory_limit * 1024);
        let (input, _) = self.task.testcase(test_index);
        self.run_box()?.exec(
            &self.language.run_command(),
            &limits,
            Some(File::open(input)?),
            Some("out.out"),
            &[],
        )
    }

    fn check(&mut self, test_index: u64) -> Result<bool> {
        let checker = self
            .language
            .custom_checker()
            .unwrap_or(&self.task.manifest().checker);
        let (input, answer) = self.task.testcase(test_index);
        check_output(
            checker,
            &input,
            &self.run_box()?.path().join("out.out"),
            &answer,
        )
    }
}

pub fn check_output(checker: &str, input: &Path, output: &Path, answer: &Path) -> Result<bool> {
    validate_checker(checker).map_err(anyhow::Error::msg)?;

    let result = Command::new("timeout")
        .arg("10")
        .arg(format!("checker/{checker}"))
        .arg(input)
        .arg(output)
        .arg(answer)
        .output()?;

    anyhow::ensure!(
        result.status.code() != Some(124),
        "Checker execution timed out after 10 seconds"
    );

    Ok(result.stdout.starts_with(b"Correct\n100")
        || (!result.stdout.starts_with(b"Incorrect")
            && result.status.success()
            && result.stdout.is_empty()))
}
