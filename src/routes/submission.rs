use crate::error::{json_response, AppError, ResponseCode};
use crate::queue::Queue;
use axum::{response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateSubmission {
    pub task_id: String,
    pub submission_id: u64,
    pub code: String,
    pub language: String,
}

pub async fn create_submission<Q: Queue>(
    Json(req): Json<CreateSubmission>,
    queue: Q,
) -> Result<impl IntoResponse, AppError> {
    queue
        .publish(
            "queue",
            req.task_id,
            req.submission_id,
            req.code,
            req.language,
        )
        .await?;

    Ok(json_response(ResponseCode::Created))
}
