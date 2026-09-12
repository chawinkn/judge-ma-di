use anyhow::{Context, Result};
use std::fs;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskConfig {
    pub time_limit: f64,
    pub memory_limit: u64,
    pub checker: String,
    pub skip: bool,
    pub full_score: u64,
    pub num_testcases: u64,
    pub subtasks: Vec<Subtask>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Subtask {
    pub full_score: u64,
    pub num_testcases: u64,
}

pub const ALLOWED_CHECKERS: &[&str] = &[
    "fcmp", "hcmp", "lcmp", "ncmp", "rcmp4", "rcmp6", "rcmp9", "wcmp", "yesno",
];

pub fn validate_checker(checker: &str) -> Result<(), AppError> {
    if !ALLOWED_CHECKERS.contains(&checker) {
        return Err(AppError::BadRequest(format!(
            "Unsupported checker '{checker}'. Allowed checkers: {}",
            ALLOWED_CHECKERS.join(", ")
        )));
    }

    Ok(())
}

pub fn validate_task_id(task_id: &str) -> Result<(), AppError> {
    if task_id.is_empty()
        || !task_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(AppError::BadRequest("invalid task_id".to_string()));
    }
    Ok(())
}

pub fn get_task_config(task_id: &str) -> Result<TaskConfig, AppError> {
    validate_task_id(task_id)?;
    let task_config_data = fs::read_to_string(format!("tasks/{task_id}/manifest.json"))
        .map_err(|_| AppError::NotFound("Task id not found".to_string()))?;
    let task_config: TaskConfig =
        serde_json::from_str(&task_config_data).context("Failed to parse manifest.json")?;
    validate_checker(&task_config.checker)?;

    if (task_config.num_testcases == 0 && task_config.subtasks.is_empty())
        || task_config.subtasks.iter().any(|s| s.num_testcases == 0)
    {
        return Err(AppError::BadRequest(
            "Task and subtasks must have at least 1 testcase".to_string(),
        ));
    }

    Ok(task_config)
}
