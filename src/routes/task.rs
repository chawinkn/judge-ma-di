use std::path::PathBuf;
use std::sync::Arc;
use axum::{ response::IntoResponse, http::StatusCode, Json, extract::Path, extract::Multipart };
use futures::StreamExt;
use serde::{ Deserialize, Serialize };
use serde_json::json;
use crate::helper::get_task_config;
use crate::AppState;
use std::fs::{ self, File };
use std::env;
use std::io::{ Cursor, Write };
use zip_extract::extract;

#[derive(Debug, Deserialize, Serialize)]
pub struct GetTask {
    task_id: String,
}

pub async fn get_task(Path(task_id): Path<String>, _state: Arc<AppState>) -> impl IntoResponse {
    let task_config = get_task_config(task_id);
    match task_config {
        Ok(_) => {}
        Err(_err) => {
            return (StatusCode::NOT_FOUND, Json(json!({ "error": "Invalid task id" })));
        }
    }
    return (StatusCode::OK, Json(json!(task_config.unwrap())));
}

pub async fn create_task(
    Path(task_id): Path<String>,
    mut multipart: Multipart
) -> impl IntoResponse {
    let dir_path = format!("tasks/{}", task_id);
    fs::create_dir_all(&dir_path).unwrap_or_else(|e| {
        eprintln!("Error creating directory: {}", e);
    });

    while let Some(mut field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
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
            zip_extract
                ::extract(Cursor::new(data), &target_dir, true)
                .expect("Error extracting zip file");
            // println!("Zip file `{}` extracted to {}", file_name, target_dir.display());

            fs::remove_file(&file_path).expect("Error deleting zip file");
            // println!("Zip file `{}` deleted", file_name);
        }

        // println!("File `{}` written to {}", file_name, file_path);
    }
    return (StatusCode::OK, Json(json!({ "message": "ok" })));
}

pub async fn delete_task(Path(task_id): Path<String>) -> impl IntoResponse {
    let dir_path = format!("tasks/{}", task_id);
    fs::remove_dir_all(&dir_path).unwrap_or_else(|e| {
        eprintln!("Error creating directory: {}", e);
    });
    return (StatusCode::OK, Json(json!({ "message": "ok" })));
}
