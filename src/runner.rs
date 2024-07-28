use std::{ fs, path::PathBuf, cmp };
use log::info;
use anyhow::Result;

use crate::helper::{ get_language_config, get_task_config };
use crate::isolate::{ Isolate, RunVerdict };
use serde::{ Deserialize, Serialize };

#[derive(Debug, Serialize, Deserialize)]
pub struct RunResult {
    pub status: String,
    pub test_index: u64,
    pub subtask_index: u64,
    pub score: u64,
    pub time: f64,
    pub memory: u64,
}

pub struct JudgeResult {
    pub result: Vec<RunResult>,
    pub status: String,
    pub score: u64,
    pub time: u64,
    pub memory: u64,
}

async fn is_testcases_error(task_path: &str, num_testcases: u64) -> bool {
    for i in 1..=num_testcases {
        let input_file = format!("{}/{}.in", task_path, i);
        let output_file = format!("{}/{}.sol", task_path, i);

        if !fs::metadata(&input_file).is_ok() || !fs::metadata(&output_file).is_ok() {
            return true;
        }
    }
    false
}

fn get_status(verdict: RunVerdict) -> String {
    match verdict {
        RunVerdict::VerdictOK => "Accepted".to_string(),
        RunVerdict::VerdictTLE => "Time Limit Exceeded".to_string(),
        RunVerdict::VerdictMLE => "Memory Limit Exceeded".to_string(),
        RunVerdict::VerdictRE => "Runtime Error".to_string(),
        RunVerdict::VerdictSG => "Signal Error".to_string(),
        RunVerdict::VerdictXX => "Internal Error".to_string(),
        _ => "".to_string(),
    }
}

pub async fn run(
    task_id: String,
    submission_id: u64,
    code: String,
    language: String
) -> Result<JudgeResult> {
    let language_config = get_language_config(&language).unwrap();
    let task_config = get_task_config(task_id.clone())?;

    let mut isolate = Isolate {
        box_path: PathBuf::new(),
        box_id: submission_id % 1000,
        time_limit: task_config.time_limit,
        memory_limit: task_config.memory_limit * 1000,
        task_id: task_id.clone(),
        code,
        ext: language_config.ext,
        compile_script: language_config.compile,
        run_script: language_config.run,
        checker: task_config.checker,
    };

    let mut judge_result = JudgeResult {
        result: vec![],
        status: "Completed".to_string(),
        score: 0,
        time: 0,
        memory: 0,
    };

    let task_path = format!("tasks/{}/testcases", task_id);

    if is_testcases_error(&task_path, task_config.num_testcases).await {
        judge_result.status = "Testcases Error".to_string();
        return Ok(judge_result);
    }

    isolate.init().await?;
    let compile_result = isolate.compile().await?;

    if compile_result.status == RunVerdict::CompilationError {
        judge_result.status = "Compilation Error".to_string();
    } else {
        let subtasks = task_config.subtasks;
        let use_skip = task_config.skip;
        let mut test_index = 1;

        if subtasks.is_empty() {
            for _ in 1..=task_config.num_testcases {
                let mut score = task_config.full_score / task_config.num_testcases;
                let isolate_result = isolate.run(test_index).await?;
                let correct =
                    isolate_result.status == RunVerdict::VerdictOK &&
                    isolate.check(test_index).await?;
                if !correct {
                    score = 0;
                }

                let status = if correct {
                    get_status(isolate_result.status)
                } else {
                    "Wrong Answer".to_string()
                };

                judge_result.score += score;
                judge_result.memory = cmp::max(judge_result.memory, isolate_result.memory_usage);
                judge_result.time = cmp::max(
                    judge_result.time,
                    (isolate_result.time_usage * 1000.0) as u64
                );

                judge_result.result.push(RunResult {
                    status,
                    test_index,
                    subtask_index: 0,
                    score,
                    time: isolate_result.time_usage,
                    memory: isolate_result.memory_usage,
                });

                test_index += 1;
            }
        } else {
            let mut subtask_index = 1;

            for subtask in subtasks {
                let mut correct_all = true;
                let mut skipped = false;
                let mut subtask_result = vec![];

                for _ in 0..subtask.num_testcases {
                    if use_skip && skipped {
                        subtask_result.push(RunResult {
                            status: "Skipped".to_string(),
                            test_index,
                            subtask_index,
                            score: 0,
                            time: 0.0,
                            memory: 0,
                        });
                    } else {
                        let isolate_result = isolate.run(test_index).await?;
                        let mut correct =
                            isolate_result.status == RunVerdict::VerdictOK &&
                            isolate.check(test_index).await?;
                        let score = if correct {
                            subtask.full_score / subtask.num_testcases
                        } else {
                            0
                        };

                        if !correct {
                            correct_all = false;
                            skipped = true;
                        }

                        let status = if correct {
                            get_status(isolate_result.status)
                        } else {
                            "Wrong Answer".to_string()
                        };

                        judge_result.memory = cmp::max(
                            judge_result.memory,
                            isolate_result.memory_usage
                        );
                        judge_result.time = cmp::max(
                            judge_result.time,
                            (isolate_result.time_usage * 1000.0) as u64
                        );

                        subtask_result.push(RunResult {
                            status,
                            test_index,
                            subtask_index,
                            score,
                            time: isolate_result.time_usage,
                            memory: isolate_result.memory_usage,
                        });
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
                subtask_index += 1;
            }
        }
    }

    let judge_result_json = serde_json::to_string(&judge_result.result)?;

    info!("{:?}", judge_result_json);

    isolate.cleanup().await?;
    Ok(judge_result)
}
