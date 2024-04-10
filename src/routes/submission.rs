use std::sync::Arc;
use axum::{ response::IntoResponse, http::StatusCode, Json };
use serde::{ Deserialize, Serialize };
use serde_json::json;
use crate::helper::write_file;
use crate::{ rbmq, AppState };

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateSubmission {
    task_id: String,
    submission_id: u64,
    code: String,
    language: String,
}

pub async fn create_submission(
    Json(req): Json<CreateSubmission>,
    state: Arc<AppState>
) -> impl IntoResponse {
    let result = write_file(&req.submission_id.to_string(), &req.code, &req.language, "temp");
    match result {
        Ok(_) => {}
        Err(_err) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": _err.to_string() })));
        }
    }

    rbmq::publish_message(
        state.channel.to_owned(),
        "queue".to_string(),
        req.task_id.clone(),
        req.submission_id,
        req.language
    ).await.expect("Unable to publish RabbitMQ message");

    return (StatusCode::CREATED, Json(json!({ "message": "success" })));
}
