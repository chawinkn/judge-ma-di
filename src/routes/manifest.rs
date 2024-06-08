use std::sync::Arc;
use axum::{ response::IntoResponse, http::StatusCode, Json, extract::Path };
use serde::{ Deserialize, Serialize };
use serde_json::json;
use crate::helper::get_task_config;
use crate::AppState;

#[derive(Debug, Deserialize, Serialize)]
pub struct GetTask {
    task_id: String,
}

pub async fn get_manifest(Path(task_id): Path<String>, _state: Arc<AppState>) -> impl IntoResponse {
    let task_config = get_task_config(task_id);
    match task_config {
        Ok(_) => {}
        Err(_err) => {
            return (StatusCode::NOT_FOUND, Json(json!({ "error": "Invalid task id" })));
        }
    }
    return (StatusCode::OK, Json(json!(task_config.unwrap())));
}
