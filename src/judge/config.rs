use anyhow::{Context, Result};
use std::env;
use std::fs;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Deserialize, Serialize)]
pub struct LanguageConfig {
    pub lang: String,
    pub ext: String,
    pub compile: String,
    pub run: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub language: Vec<LanguageConfig>,
}

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

pub fn get_config() -> Result<Config> {
    let current_dir = env::current_dir()?;
    let config_path = current_dir.join("config.json");
    let config_data = fs::read_to_string(config_path).context("Failed to read config.json")?;
    let config = serde_json::from_str(&config_data).context("Failed to parse config.json")?;

    Ok(config)
}

pub fn get_language_config(language: &str) -> Result<LanguageConfig, AppError> {
    let config = get_config().context("Failed to get config")?;

    config
        .language
        .into_iter()
        .find(|lang_config| lang_config.lang == language)
        .ok_or_else(|| AppError::BadRequest("Unsupported Language".to_string()))
}

pub fn get_task_config(task_id: &str) -> Result<TaskConfig, AppError> {
    let current_dir = env::current_dir()?;
    let task_config_path = current_dir
        .join("tasks")
        .join(task_id)
        .join("manifest.json");
    let task_config_data = fs::read_to_string(task_config_path)
        .context("Failed to read manifest.json")
        .map_err(|_| AppError::NotFound("Task id not found".to_string()))?;
    let task_config =
        serde_json::from_str(&task_config_data).context("Failed to parse manifest.json")?;

    Ok(task_config)
}
