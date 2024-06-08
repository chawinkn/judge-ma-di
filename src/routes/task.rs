use std::path::PathBuf;
use std::sync::Arc;
use axum::{
    response::IntoResponse,
    response::Response,
    http::StatusCode,
    Json,
    http::header,
    extract::Path,
    extract::Multipart,
};
use futures::StreamExt;
use serde_json::json;
use crate::AppState;
use std::fs::{ self, File };
use std::io::{ Cursor, Write };
use std::env;
use tokio::io::AsyncReadExt;

pub async fn get_task_testcases(
    Path(task_id): Path<String>,
    _state: Arc<AppState>
) -> impl IntoResponse {
    let current_dir = env::current_dir().unwrap();
    let path = current_dir.join("tasks").join(task_id).join("testcases.zip");

    match tokio::fs::File::open(&path).await {
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
                .header(header::CONTENT_TYPE, "application/zip")
                .header(header::CONTENT_DISPOSITION, "inline; filename=\"testcases.zip\"")
                .body(contents.into())
                .unwrap()
        }
        Err(_) => {
            (StatusCode::NOT_FOUND, Json(json!({ "error": "File not found" }))).into_response()
        }
    }
}

pub async fn upload_task(
    Path(task_id): Path<String>,
    mut multipart: Multipart
) -> impl IntoResponse {
    let dir_path = format!("tasks/{}", task_id);
    fs::create_dir_all(&dir_path).unwrap_or_else(|e| {
        eprintln!("Error creating directory: {}", e);
    });

    while let Some(mut field) = multipart.next_field().await.unwrap() {
        let file_name = field.file_name().unwrap().to_string();
        let file_path = format!("{}/{}", dir_path, file_name);
        let mut file = File::create(&file_path).expect("Error creating file");

        let mut data = vec![];
        while let Some(chunk) = field.next().await {
            let chunk_data = chunk.unwrap();
            file.write_all(&chunk_data).expect("Error writing to file");
            data.extend_from_slice(&chunk_data);
        }

        if file_name.ends_with(".zip") {
            let target_dir = PathBuf::from(format!("{}/{}", dir_path, "testcases"));
            if target_dir.exists() {
                fs::remove_dir_all(&target_dir).unwrap_or_else(|e| {
                    eprintln!("Error deleting directory: {}", e);
                });
            }
            zip_extract
                ::extract(Cursor::new(data), &target_dir, true)
                .expect("Error extracting zip file");
        }
    }
    return (StatusCode::OK, Json(json!({ "message": "ok" })));
}

pub async fn delete_task(Path(task_id): Path<String>) -> impl IntoResponse {
    let dir_path = format!("tasks/{}", task_id);
    fs::remove_dir_all(&dir_path).unwrap_or_else(|e| {
        eprintln!("Error deleting directory: {}", e);
    });
    return (StatusCode::OK, Json(json!({ "message": "ok" })));
}
