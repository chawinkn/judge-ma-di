use axum::{
    extract::{Multipart, Path},
    http::header,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::io::Cursor;
use std::path::PathBuf;

use crate::error::{json_response, AppError, ResponseCode};
use crate::judge::config::get_task_config;

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

pub async fn get_task_testcases(
    Path(task_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    validate_task_id(&task_id)?;
    let contents = tokio::fs::read(format!("tasks/{task_id}/testcases.zip"))
        .await
        .map_err(|_| AppError::NotFound(format!("Testcases for task '{task_id}' not found")))?;

    Ok((
        [
            (header::CONTENT_TYPE, "application/zip"),
            (
                header::CONTENT_DISPOSITION,
                "inline; filename=\"testcases.zip\"",
            ),
        ],
        contents,
    ))
}

pub async fn upload_task(
    Path(task_id): Path<String>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    validate_task_id(&task_id)?;
    let dir = format!("tasks/{task_id}");
    tokio::fs::create_dir_all(&dir).await?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let raw_name = field
            .file_name()
            .ok_or_else(|| AppError::BadRequest("missing filename".to_string()))?;
        let safe_name = std::path::Path::new(raw_name)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| AppError::BadRequest("invalid filename".to_string()))?
            .to_string();
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        tokio::fs::write(format!("{dir}/{safe_name}"), &data).await?;

        if safe_name.ends_with(".zip") {
            let target = format!("{dir}/testcases");
            let _ = tokio::fs::remove_dir_all(&target).await;
            zip_extract::extract(Cursor::new(&data), &PathBuf::from(target), true)
                .map_err(|e| AppError::BadRequest(format!("Invalid zip archive: {e}")))?;
        }
    }

    Ok(json_response(ResponseCode::Ok))
}

pub async fn delete_task(Path(task_id): Path<String>) -> Result<impl IntoResponse, AppError> {
    validate_task_id(&task_id)?;
    tokio::fs::remove_dir_all(format!("tasks/{task_id}"))
        .await
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                AppError::NotFound(format!("Task '{task_id}' not found"))
            }
            _ => AppError::Internal(e.into()),
        })?;
    Ok(json_response(ResponseCode::Ok))
}

pub async fn get_manifest(Path(task_id): Path<String>) -> Result<impl IntoResponse, AppError> {
    validate_task_id(&task_id)?;
    let task_config = get_task_config(&task_id)?;
    Ok((StatusCode::OK, Json(task_config)))
}

pub async fn get_desc(Path(task_id): Path<String>) -> Result<impl IntoResponse, AppError> {
    validate_task_id(&task_id)?;
    let contents = tokio::fs::read(format!("tasks/{task_id}/desc.pdf"))
        .await
        .map_err(|_| AppError::NotFound(format!("Description for task '{task_id}' not found")))?;

    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (header::CONTENT_DISPOSITION, "inline; filename=\"desc.pdf\""),
        ],
        contents,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_task_id_valid() {
        assert!(validate_task_id("a_plus_b").is_ok());
        assert!(validate_task_id("task-123").is_ok());
        assert!(validate_task_id("0").is_ok());
        assert!(validate_task_id("TASK").is_ok());
    }

    #[test]
    fn test_validate_task_id_invalid() {
        assert!(validate_task_id("").is_err());
        assert!(validate_task_id("../etc").is_err());
        assert!(validate_task_id("task/1").is_err());
        assert!(validate_task_id("task\\1").is_err());
        assert!(validate_task_id("task.name").is_err());
        assert!(validate_task_id("task name").is_err());
    }

    #[test]
    fn test_filename_sanitization() {
        let extract_safe_name = |raw: &str| {
            std::path::Path::new(raw)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        };

        assert_eq!(
            extract_safe_name("../../evil.txt"),
            Some("evil.txt".to_string())
        );
        assert_eq!(extract_safe_name("/etc/passwd"), Some("passwd".to_string()));
        assert_eq!(
            extract_safe_name("manifest.json"),
            Some("manifest.json".to_string())
        );
        assert_eq!(extract_safe_name(".."), None);
        assert_eq!(extract_safe_name("."), None);
        assert_eq!(extract_safe_name(""), None);
    }
}
