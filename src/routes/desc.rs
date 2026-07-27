use crate::error::{json_response, ResponseCode};
use axum::{
    extract::Path, http::header, http::StatusCode, response::IntoResponse, response::Response,
};
use std::env;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn get_desc(Path(task_id): Path<String>) -> impl IntoResponse {
    let current_dir = env::current_dir().unwrap();
    let path = current_dir.join("tasks").join(task_id).join("desc.pdf");

    match File::open(&path).await {
        Ok(mut file) => {
            let mut contents = Vec::new();
            if (file.read_to_end(&mut contents).await).is_err() {
                return json_response(ResponseCode::InternalServerError);
            }

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/pdf")
                .header(header::CONTENT_DISPOSITION, "inline; filename=\"desc.pdf\"")
                .body(contents.into())
                .unwrap()
        }
        Err(_) => json_response(ResponseCode::NotFound),
    }
}
