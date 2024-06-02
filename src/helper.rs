use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::env;
use anyhow::Result;

use serde::{ Deserialize, Serialize };

#[derive(Debug, Deserialize, Serialize)]
pub struct LanguageConfig {
    pub ext: String,
    pub compile: String,
    pub run: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub language: HashMap<String, LanguageConfig>,
    pub judge: JudgeConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JudgeConfig {
    pub max_worker: u64,
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
    let config_data = fs::read_to_string(config_path)?;
    let config = serde_json::from_str(&config_data)?;

    Ok(config)
}

pub fn get_judge_config() -> Result<JudgeConfig> {
    let config = get_config()?;
    let judge_config = config.judge;

    Ok(judge_config)
}

pub fn get_language_config(language: &str) -> Result<LanguageConfig, Box<dyn Error>> {
    let config = get_config()?;
    let language_config = config.language;

    match language_config.get(language) {
        Some(config) =>
            Ok(LanguageConfig {
                ext: config.ext.clone(),
                compile: config.compile.clone(),
                run: config.run.clone(),
            }),
        None => Err("Unsupported language".into()),
    }
}

pub fn get_task_config(task_id: String) -> Result<TaskConfig> {
    let current_dir = env::current_dir()?;
    let task_config_path = current_dir.join("tasks").join(task_id).join("manifest.json");
    let task_config_data = fs::read_to_string(task_config_path)?;
    let task_config = serde_json::from_str(&task_config_data)?;

    Ok(task_config)
}
