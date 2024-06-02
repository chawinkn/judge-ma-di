use axum::{
    response::IntoResponse,
    response::Response,
    http::StatusCode,
    Json,
    http::header,
    extract::Path,
};
use serde::{ Deserialize, Serialize };
use serde_json::json;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use std::env;
use std::sync::Arc;
use crate::AppState;

#[derive(Debug, Deserialize, Serialize)]
pub struct GetTask {
    task_id: String,
}

pub async fn get_desc(Path(task_id): Path<String>, _state: Arc<AppState>) -> impl IntoResponse {
    let current_dir = env::current_dir().unwrap();
    let path = current_dir.join("tasks").join(task_id).join("desc.pdf");

    match File::open(&path).await {
        Ok(mut file) => {
            let mut contents = Vec::new();
            if let Err(_) = file.read_to_end(&mut contents).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Failed to read file" })),
                ).into_response();
            }

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/pdf")
                .header(header::CONTENT_DISPOSITION, "inline; filename=\"desc.pdf\"")
                .body(contents.into())
                .unwrap()
        }
        Err(_) => {
            (StatusCode::NOT_FOUND, Json(json!({ "error": "File not found" }))).into_response()
        }
    }
}
