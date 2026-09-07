use crate::error::AppError;
use axum::{extract::Path, http::header, response::IntoResponse};

pub async fn get_desc(Path(task_id): Path<String>) -> Result<impl IntoResponse, AppError> {
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
