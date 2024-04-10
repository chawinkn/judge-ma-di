use std::{ env, fs, path::PathBuf };
use log::info;
use anyhow::Result;

use crate::helper::{ get_language_config, get_task_config };
use crate::isolate::Isolate;
use crate::isolate::RunVerdict;

pub async fn run(
    consumer_id: u64,
    task_id: String,
    submission_id: u64,
    language: String
) -> Result<()> {
    let language_config = get_language_config(&language).unwrap();
    let task_config = get_task_config(task_id.clone()).unwrap();

    let mut isolate = Isolate {
        box_path: PathBuf::new(),
        box_id: consumer_id,
        time_limit: task_config.time_limit,
        memory_limit: task_config.memory_limit * 1000,
        input_path: PathBuf::from(format!("{}.{}", submission_id, language_config.ext)),
        task_id: task_id.clone(),
        ext: language_config.ext,
        compile_script: language_config.compile,
        run_script: language_config.run,
        checker: task_config.checker,
    };

    isolate.init().await?;

    let compile_result = isolate.compile().await?;

    let mut score = 0;
    let mut total_testcases = 0;

    let current_dir = env::current_dir()?;
    let task_path = current_dir.join("tasks").join(task_id).join("testcases");

    if compile_result.status == RunVerdict::VerdictOK {
        let subtasks = task_config.subtasks;
        let mut test_index = 1;
        if subtasks.len() == 0 {
            for entry in fs::read_dir(task_path)? {
                let entry = entry?;
                let path = entry.path();

                let ext = path.extension().unwrap();
                if ext == "in" {
                    let run_result = isolate.run(test_index).await?;
                    if run_result.status == RunVerdict::VerdictOK {
                        let correct = isolate.check(test_index).await?;
                        if correct {
                            score += 1;
                        }
                    }
                    test_index += 1;
                    total_testcases += 1;
                }
            }
        } else {
            for subtask in subtasks {
                let mut correct_all = true;
                let mut skipped = false;
                total_testcases += subtask.num_testcases;

                for _i in 0..subtask.num_testcases {
                    if !skipped {
                        let run_result = isolate.run(test_index).await?;
                        if run_result.status == RunVerdict::VerdictOK {
                            let correct = isolate.check(test_index).await?;
                            if !correct {
                                correct_all = false;
                                skipped = true;
                            }
                        }
                    }
                    test_index += 1;
                }
                if correct_all {
                    score += subtask.num_testcases;
                }
            }
        }
    }

    info!(" [x] Submission id : {} Result : {} / {}", submission_id, score, total_testcases);

    isolate.cleanup().await?;

    Ok(())
}
