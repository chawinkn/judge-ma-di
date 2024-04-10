use std::collections::HashMap;
use std::error::Error;
use std::fs::{ self, create_dir, File };
use std::io::Write;
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

#[derive(Debug, Deserialize)]
pub struct TaskConfig {
    pub id: String,
    pub time_limit: f64,
    pub memory_limit: u64,
    pub checker: String,
    pub skip: bool,
    pub subtasks: Vec<Subtask>,
}

#[derive(Debug, Deserialize)]
pub struct Subtask {
    pub full_score: u64,
    pub num_testcases: u64,
}

pub fn write_file(name: &str, content: &str, language: &str, path: &str) -> std::io::Result<()> {
    let language_config = match get_language_config(language) {
        Ok(ext) => ext,
        Err(err) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", err)));
        }
    };

    let current_dir = env::current_dir()?;
    let destination_path = current_dir.join(path).join(format!("{}.{}", name, language_config.ext));

    if let Some(parent_dir) = destination_path.parent() {
        if !parent_dir.exists() {
            create_dir(parent_dir)?;
        }
    }

    let mut file = File::create(destination_path)?;
    file.write_all(content.as_bytes())?;

    Ok(())
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
