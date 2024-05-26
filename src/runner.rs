use std::{ env, fs, path::PathBuf, cmp };
use log::info;
use anyhow::Result;

use crate::helper::{ get_language_config, get_task_config };
use crate::isolate::Isolate;
use crate::isolate::RunVerdict;

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

fn get_status(verdict: RunVerdict) -> String {
    if verdict == RunVerdict::VerdictOK {
        return "Accepted".to_string();
    } else if verdict == RunVerdict::VerdictTLE {
        return "Time Limit Exceeded".to_string();
    } else if verdict == RunVerdict::VerdictMLE {
        return "Memory Limit Exceeded".to_string();
    } else if verdict == RunVerdict::VerdictRE {
        return "Runtime Error".to_string();
    } else if verdict == RunVerdict::VerdictSG {
        return "Signal Error".to_string();
    } else if verdict == RunVerdict::VerdictXX {
        return "Internal Error".to_string();
    }
    return "".to_string();
}

pub async fn run(
    task_id: String,
    submission_id: u64,
    code: String,
    language: String
) -> Result<JudgeResult> {
    let language_config = get_language_config(&language).unwrap();
    let task_config = get_task_config(task_id.clone()).unwrap();

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

    let mut judge_result: JudgeResult = JudgeResult {
        result: vec![],
        status: "Completed".to_string(),
        score: 0,
        time: 0,
        memory: 0,
    };
    let mut run_result: Vec<RunResult> = vec![];

    isolate.init().await?;

    let compile_result = isolate.compile().await?;

    let mut score;

    let current_dir = env::current_dir()?;
    let task_path = current_dir.join("tasks").join(task_id).join("testcases");
    if compile_result.status == RunVerdict::CompilationError {
        judge_result.status = "Compilation Error".to_string();
    } else {
        let subtasks = task_config.subtasks;
        let use_skip = task_config.skip;
        let mut test_index = 1;
        if subtasks.len() == 0 {
            for entry in fs::read_dir(task_path)? {
                let entry = entry?;
                let path = entry.path();

                let ext = path.extension().unwrap();
                if ext != "in" {
                    continue;
                }
                score = task_config.full_score / task_config.num_testcases;
                let isolate_result = isolate.run(test_index).await?;
                let mut correct = true;
                if isolate_result.status == RunVerdict::VerdictOK {
                    let checker_result = isolate.check(test_index).await?;
                    if !checker_result {
                        score = 0;
                        correct = false;
                    }
                } else {
                    score = 0;
                }
                let mut status = get_status(isolate_result.status);
                if !correct {
                    status = "Wrong Answer".to_string();
                }
                judge_result.score += score;
                judge_result.memory = cmp::max(judge_result.memory, isolate_result.memory_usage);
                judge_result.time = cmp::max(
                    judge_result.time,
                    (isolate_result.time_usage * 1000.0) as u64
                );
                run_result.push(RunResult {
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
            let mut subtask_result: Vec<RunResult> = vec![];
            for subtask in subtasks {
                let mut correct_all = true;
                let mut skipped = false;

                for _i in 0..subtask.num_testcases {
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
                        let mut correct = true;
                        score = subtask.full_score / subtask.num_testcases;
                        if isolate_result.status == RunVerdict::VerdictOK {
                            let checker_result = isolate.check(test_index).await?;
                            if !checker_result {
                                correct_all = false;
                                skipped = true;
                                correct = false;
                                score = 0;
                            }
                        } else {
                            correct_all = false;
                            skipped = true;
                            score = 0;
                        }
                        let mut status = get_status(isolate_result.status);
                        if !correct {
                            status = "Wrong Answer".to_string();
                        }
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
                if !correct_all {
                    for subtask_result in &mut subtask_result {
                        subtask_result.score = 0;
                    }
                } else {
                    judge_result.score += subtask.full_score;
                }
                run_result.append(&mut subtask_result);
                subtask_index += 1;
            }
        }
    }

    let judge_result_json = serde_json::to_string(&run_result)?;

    judge_result.result = run_result;

    info!("{:?}", judge_result_json);

    isolate.cleanup().await?;

    Ok(judge_result)
}
