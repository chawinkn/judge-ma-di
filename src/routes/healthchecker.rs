use axum::{ response::IntoResponse, http::StatusCode, Json };
use serde_json::json;

pub async fn health_checker() -> impl IntoResponse {
    return (StatusCode::OK, Json(json!({ "message": "OK" })));
}
