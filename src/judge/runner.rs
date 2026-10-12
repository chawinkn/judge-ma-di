use anyhow::Result;
use std::fmt;

use crate::judge::isolate::RunVerdict;
use crate::judge::languages::{get_language, Language};
use crate::judge::sandbox::{IsolateSandbox, Sandbox};
use crate::judge::task::{Subtask, Task};
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

impl JudgeResult {
    fn absorb(&mut self, run: &RunResult) {
        self.memory = self.memory.max(run.memory);
        self.time = self.time.max((run.time * 1000.0) as u64);
    }
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

pub fn run(task_id: &str, submission_id: u64, code: String, language: &str) -> Result<JudgeResult> {
    let language = get_language(language).map_err(anyhow::Error::from)?;
    let task = Task::open(task_id).map_err(anyhow::Error::from)?;
    let mut sandbox = IsolateSandbox::new(submission_id % 500, &task, language, code);
    judge(&task, language, &mut sandbox)
}

pub fn judge(
    task: &Task,
    language: &dyn Language,
    sandbox: &mut impl Sandbox,
) -> Result<JudgeResult> {
    let manifest = task.manifest();

    if !manifest.allowed_languages.is_empty()
        && !manifest
            .allowed_languages
            .iter()
            .any(|l| l == language.name())
    {
        return Err(crate::error::AppError::BadRequest(format!(
            "Language '{}' is not allowed for task '{}'. Allowed languages: {}",
            language.name(),
            task.id(),
            manifest.allowed_languages.join(", ")
        ))
        .into());
    }

    if !task.has_all_testcases() {
        return Ok(JudgeResult {
            status: JudgeStatus::TestcasesError,
            ..Default::default()
        });
    }

    if !sandbox.compile()? {
        return Ok(JudgeResult {
            status: JudgeStatus::CompilationError,
            ..Default::default()
        });
    }

    if manifest.subtasks.is_empty() {
        run_normal(sandbox, manifest.full_score, manifest.num_testcases)
    } else {
        run_subtask(sandbox, &manifest.subtasks, manifest.skip)
    }
}

fn run_each<S: Sandbox>(
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

fn run_normal<S: Sandbox>(
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
        judge_result.absorb(&run_result);

        judge_result.result.push(run_result);
    }

    Ok(judge_result)
}

fn run_subtask<S: Sandbox>(
    isolate: &mut S,
    subtasks: &[Subtask],
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

    for (subtask_index, subtask) in (1..).zip(subtasks) {
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

                judge_result.absorb(&run_result);

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
