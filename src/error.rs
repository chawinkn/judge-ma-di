use std::fmt;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tracing::error;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    NotFound(String),
    Internal(anyhow::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::BadRequest(msg) => write!(f, "{msg}"),
            AppError::NotFound(msg) => write!(f, "{msg}"),
            AppError::Internal(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for AppError {}

/// Enum instead of a raw `StatusCode` match so the HTTP status and its body
/// code can never drift apart (no silent `_ => INTERNAL_SERVER_ERROR` catch-all).
pub enum ResponseCode {
    Ok,
    Created,
    BadRequest,
    NotFound,
    ServiceUnavailable,
    InternalServerError,
}

impl ResponseCode {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            ResponseCode::Ok => (StatusCode::OK, "OK"),
            ResponseCode::Created => (StatusCode::CREATED, "CREATED"),
            ResponseCode::BadRequest => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            ResponseCode::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            ResponseCode::ServiceUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "SERVICE_UNAVAILABLE")
            }
            ResponseCode::InternalServerError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_SERVER_ERROR")
            }
        }
    }
}

pub fn json_response(code: ResponseCode) -> Response {
    let (status, code) = code.status_and_code();
    (status, Json(json!({ "code": code }))).into_response()
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::BadRequest(_) => json_response(ResponseCode::BadRequest),
            AppError::NotFound(_) => json_response(ResponseCode::NotFound),
            AppError::Internal(err) => {
                error!("Internal Server Error: {:?}", err);
                json_response(ResponseCode::InternalServerError)
            }
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(err.into())
    }
}
