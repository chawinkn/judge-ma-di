use anyhow::Result;
use std::{
    fmt,
    fs::{self, File},
    path::Path,
    process::Command,
};

use crate::judge::isolate::{IsolateBox, IsolateResult, Limits, RunVerdict, Sandbox};
use crate::judge::languages::{get_language, Language};
use crate::judge::task::{validate_checker, Subtask, Task};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RunResult {
    pub status: RunStatus,
    pub test_index: u64,
    pub subtask_index: u64,
    pub score: u64,
    pub time: f64,
    pub memory: u64,
}

#[derive(Debug)]
pub struct JudgeResult {
    pub result: Vec<RunResult>,
    pub status: JudgeStatus,
    pub score: u64,
    pub time: u64,
    pub memory: u64,
}

impl Default for JudgeResult {
    fn default() -> Self {
        Self {
            result: Vec::new(),
            status: JudgeStatus::Completed,
            score: 0,
            time: 0,
            memory: 0,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum JudgeStatus {
    Completed,
    CompilationError,
    TestcasesError,
}

impl fmt::Display for JudgeStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Completed => write!(f, "Completed"),
            Self::CompilationError => write!(f, "Compilation Error"),
            Self::TestcasesError => write!(f, "Testcases Error"),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunStatus {
    Verdict(RunVerdict),
    #[serde(rename = "Wrong Answer")]
    WrongAnswer,
    #[serde(rename = "Skip")]
    Skipped,
}

pub struct IsolateSandbox<'a> {
    run_box: IsolateBox,
    limits: Limits,
    language: &'static dyn Language,
    task: &'a Task,
}

impl Sandbox for IsolateSandbox<'_> {
    fn run(&mut self, test_index: u64) -> Result<IsolateResult> {
        let (input, _) = self.task.testcase(test_index);
        let input = File::open(input)?;
        self.run_box.exec(
            &self.language.run_command(),
            &self.limits,
            Some(input),
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
            &self.run_box.path().join("out.out"),
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

fn compile(
    run_box: &IsolateBox,
    compile_box_id: u64,
    language: &dyn Language,
    code: &str,
) -> Result<bool> {
    let Some(compile_command) = language.compile_command() else {
        return Ok(true);
    };

    let compile_box = IsolateBox::open(compile_box_id)?;
    compile_box.write(&language.source_filename(), code)?;

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

    if let Some(artifact) = language.compiled_artifact() {
        let compiled_bin = compile_box.path().join(artifact);
        if compiled_bin.exists() {
            fs::copy(&compiled_bin, run_box.path().join(artifact))?;
        }
    }

    Ok(true)
}

pub fn run(task_id: &str, submission_id: u64, code: String, language: &str) -> Result<JudgeResult> {
    let language_adapter = get_language(language).map_err(anyhow::Error::from)?;
    let task = Task::open(task_id).map_err(anyhow::Error::from)?;
    let manifest = task.manifest();

    if !manifest.allowed_languages.is_empty()
        && !manifest.allowed_languages.iter().any(|l| l == language)
    {
        return Err(crate::error::AppError::BadRequest(format!(
            "Language '{language}' is not allowed for task '{task_id}'. Allowed languages: {}",
            manifest.allowed_languages.join(", ")
        ))
        .into());
    }

    if !task.has_all_testcases() {
        let judge_result = JudgeResult {
            status: JudgeStatus::TestcasesError,
            ..Default::default()
        };

        return Ok(judge_result);
    }

    let box_id = submission_id % 500;
    let run_box = IsolateBox::open(box_id)?;
    run_box.write(&language_adapter.source_filename(), &code)?;

    if !compile(&run_box, box_id + 500, language_adapter, &code)? {
        let judge_result = JudgeResult {
            status: JudgeStatus::CompilationError,
            ..Default::default()
        };

        return Ok(judge_result);
    }

    let mut sandbox = IsolateSandbox {
        run_box,
        limits: Limits::run(manifest.time_limit, manifest.memory_limit * 1000),
        language: language_adapter,
        task: &task,
    };

    let judge_result = if manifest.subtasks.is_empty() {
        run_normal(&mut sandbox, manifest.full_score, manifest.num_testcases)
    } else {
        run_subtask(&mut sandbox, manifest.subtasks.clone(), manifest.skip)
    }?;

    Ok(judge_result)
}

pub fn run_each<S: Sandbox>(
    isolate: &mut S,
    mut score: u64,
    subtask_index: u64,
    test_index: u64,
) -> Result<RunResult> {
    let mut correct = true;

    let isolate_result = isolate.run(test_index)?;
    if isolate_result.status == RunVerdict::VerdictOK {
        if !isolate.check(test_index)? {
            score = 0;
            correct = false;
        }
    } else {
        score = 0;
    }

    let status = if correct {
        RunStatus::Verdict(isolate_result.status)
    } else {
        RunStatus::WrongAnswer
    };

    Ok(RunResult {
        status,
        test_index,
        subtask_index,
        score,
        time: isolate_result.time_usage,
        memory: isolate_result.memory_usage,
    })
}

pub fn run_normal<S: Sandbox>(
    isolate: &mut S,
    full_score: u64,
    num_testcases: u64,
) -> Result<JudgeResult> {
    if num_testcases == 0 {
        return Ok(JudgeResult::default());
    }

    let mut judge_result = JudgeResult {
        result: Vec::with_capacity(num_testcases as usize),
        ..Default::default()
    };
    let score = full_score / num_testcases;

    for test_index in 1..=num_testcases {
        let run_result = run_each(isolate, score, 0, test_index)?;

        judge_result.score += run_result.score;
        judge_result.memory = judge_result.memory.max(run_result.memory);
        judge_result.time = judge_result.time.max((run_result.time * 1000.0) as u64);

        judge_result.result.push(run_result);
    }

    Ok(judge_result)
}

pub fn run_subtask<S: Sandbox>(
    isolate: &mut S,
    subtasks: Vec<Subtask>,
    use_skip: bool,
) -> Result<JudgeResult> {
    let total_result = subtasks
        .iter()
        .map(|subtask| subtask.num_testcases as usize)
        .sum();
    let mut judge_result = JudgeResult {
        result: Vec::with_capacity(total_result),
        ..Default::default()
    };

    let mut test_index = 1;

    for (subtask_index, subtask) in (1..).zip(&subtasks) {
        if subtask.num_testcases == 0 {
            continue;
        }
        let mut correct_all = true;
        let mut skipped = false;
        let mut subtask_result = Vec::with_capacity(subtask.num_testcases as usize);
        let score = subtask.full_score / subtask.num_testcases;

        for _ in 0..subtask.num_testcases {
            if use_skip && skipped {
                subtask_result.push(RunResult {
                    status: RunStatus::Skipped,
                    test_index,
                    subtask_index,
                    score: 0,
                    time: 0.0,
                    memory: 0,
                });
            } else {
                let run_result = run_each(isolate, score, subtask_index, test_index)?;

                if run_result.score == 0 {
                    correct_all = false;
                    skipped = true;
                }

                judge_result.memory = judge_result.memory.max(run_result.memory);
                judge_result.time = judge_result.time.max((run_result.time * 1000.0) as u64);

                subtask_result.push(run_result);
            }
            test_index += 1;
        }

        if correct_all {
            judge_result.score += subtask.full_score;
        } else {
            for result in &mut subtask_result {
                result.score = 0;
            }
        }
        judge_result.result.append(&mut subtask_result);
    }

    Ok(judge_result)
}
