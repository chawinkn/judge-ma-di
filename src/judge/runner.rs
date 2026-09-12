use anyhow::Result;
use std::{
    fmt,
    path::{Path, PathBuf},
};

use crate::judge::config::{get_language_config, get_task_config, Subtask};
use crate::judge::isolate::{Isolate, RunVerdict, Sandbox};
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

pub fn is_testcases_error(task_path: &Path, num_testcases: u64) -> bool {
    num_testcases == 0
        || (1..=num_testcases).any(|i| {
            !task_path.join(format!("{i}.in")).exists()
                || !task_path.join(format!("{i}.sol")).exists()
        })
}

pub fn run(task_id: &str, submission_id: u64, code: String, language: &str) -> Result<JudgeResult> {
    let language_config = get_language_config(language).map_err(anyhow::Error::from)?;
    let task_config = get_task_config(task_id).map_err(anyhow::Error::from)?;

    let testcases_dir = PathBuf::from(format!("tasks/{task_id}/testcases"));

    if is_testcases_error(&testcases_dir, task_config.num_testcases) {
        let judge_result = JudgeResult {
            status: JudgeStatus::TestcasesError,
            ..Default::default()
        };

        return Ok(judge_result);
    }

    let mut isolate = Isolate {
        box_path: PathBuf::new(),
        box_id: submission_id % 500,
        time_limit: task_config.time_limit,
        memory_limit: task_config.memory_limit * 1000,
        code,
        ext: language_config.ext.clone(),
        compile_script: language_config.compile.clone(),
        run_script: language_config.run.clone(),
        checker: task_config.checker,
        testcases_dir,
        initialized: false,
    };

    isolate.init()?;
    let compile_result = isolate.compile()?;

    if compile_result.status == RunVerdict::CompilationError {
        let judge_result = JudgeResult {
            status: JudgeStatus::CompilationError,
            ..Default::default()
        };

        return Ok(judge_result);
    }

    let subtasks = task_config.subtasks;
    let use_skip = task_config.skip;

    let judge_result = if subtasks.is_empty() {
        run_normal(
            &mut isolate,
            task_config.full_score,
            task_config.num_testcases,
        )
    } else {
        run_subtask(&mut isolate, subtasks, use_skip)
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
