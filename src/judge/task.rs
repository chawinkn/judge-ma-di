use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const TASKS_ROOT: &str = "tasks";

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

fn validate_task_id(task_id: &str) -> Result<(), AppError> {
    if task_id.is_empty()
        || !task_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(AppError::BadRequest("invalid task_id".to_string()));
    }
    Ok(())
}

pub fn task_dir(id: &str) -> Result<PathBuf, AppError> {
    validate_task_id(id)?;
    Ok(Path::new(TASKS_ROOT).join(id))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Subtask {
    pub full_score: u64,
    pub num_testcases: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub time_limit: f64,
    pub memory_limit: u64,
    pub checker: String,
    pub skip: bool,
    pub full_score: u64,
    pub num_testcases: u64,
    pub subtasks: Vec<Subtask>,
    #[serde(default)]
    pub allowed_languages: Vec<String>,
}

impl Manifest {
    pub fn parse(bytes: &[u8]) -> Result<Self, AppError> {
        let manifest: Manifest = serde_json::from_slice(bytes)
            .map_err(|e| AppError::BadRequest(format!("Invalid manifest.json: {e}")))?;
        validate_checker(&manifest.checker)?;

        if manifest.subtasks.is_empty() && manifest.num_testcases == 0 {
            return Err(AppError::BadRequest(
                "num_testcases must be greater than 0".to_string(),
            ));
        }
        if let Some(i) = manifest.subtasks.iter().position(|s| s.num_testcases == 0) {
            return Err(AppError::BadRequest(format!(
                "subtask {} num_testcases must be greater than 0",
                i + 1
            )));
        }

        Ok(manifest)
    }

    pub fn testcase_count(&self) -> u64 {
        if self.subtasks.is_empty() {
            self.num_testcases
        } else {
            self.subtasks.iter().map(|s| s.num_testcases).sum()
        }
    }
}

pub struct Task {
    id: String,
    dir: PathBuf,
    manifest: Manifest,
}

impl Task {
    pub fn open(id: &str) -> Result<Self, AppError> {
        Self::open_in(Path::new(TASKS_ROOT), id)
    }

    pub fn open_in(root: &Path, id: &str) -> Result<Self, AppError> {
        validate_task_id(id)?;
        let dir = root.join(id);
        let bytes = fs::read(dir.join("manifest.json"))
            .map_err(|_| AppError::NotFound("Task id not found".to_string()))?;
        let manifest = Manifest::parse(&bytes)?;
        Ok(Self {
            id: id.to_string(),
            dir,
            manifest,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn into_manifest(self) -> Manifest {
        self.manifest
    }

    pub fn testcase(&self, n: u64) -> (PathBuf, PathBuf) {
        let dir = self.dir.join("testcases");
        (dir.join(format!("{n}.in")), dir.join(format!("{n}.sol")))
    }

    pub fn has_all_testcases(&self) -> bool {
        let count = self.manifest.testcase_count();
        count > 0
            && (1..=count).all(|n| {
                let (input, solution) = self.testcase(n);
                input.exists() && solution.exists()
            })
    }
}
