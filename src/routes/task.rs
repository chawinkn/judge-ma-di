use axum::{
    extract::{Multipart, Path},
    http::header,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use std::io::Cursor;
use std::path::PathBuf;

use crate::error::{json_response, AppError, ResponseCode};
use crate::judge::config::get_task_config;

pub async fn get_task_testcases(
    Path(task_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
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
    let dir = format!("tasks/{task_id}");
    tokio::fs::create_dir_all(&dir).await?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field
            .file_name()
            .ok_or_else(|| AppError::BadRequest("missing filename".to_string()))?
            .to_string();
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        tokio::fs::write(format!("{dir}/{name}"), &data).await?;

        if name.ends_with(".zip") {
            let target = format!("{dir}/testcases");
            let _ = tokio::fs::remove_dir_all(&target).await;
            zip_extract::extract(Cursor::new(&data), &PathBuf::from(target), true)
                .map_err(|e| AppError::BadRequest(format!("Invalid zip archive: {e}")))?;
        }
    }

    Ok(json_response(ResponseCode::Ok))
}

pub async fn delete_task(Path(task_id): Path<String>) -> Result<impl IntoResponse, AppError> {
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
    let task_config = get_task_config(&task_id)?;
    Ok((StatusCode::OK, Json(json!(task_config))))
}
