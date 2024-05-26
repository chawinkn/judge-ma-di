use std::sync::Arc;
use axum::{ response::IntoResponse, http::StatusCode, Json };
use serde::{ Deserialize, Serialize };
use serde_json::json;
use crate::helper::get_language_config;
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
    match get_language_config(&req.language) {
        Ok(_) => {}
        Err(_err) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": _err.to_string() })));
        }
    }

    rbmq::publish_message(
        state.channel.to_owned(),
        "queue".to_string(),
        req.task_id,
        req.submission_id,
        req.code,
        req.language
    ).await.expect("Unable to publish RabbitMQ message");

    return (StatusCode::CREATED, Json(json!({ "message": "success" })));
}
