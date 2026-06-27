use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[allow(dead_code)]
    #[error("not found: {0}")]
    NotFound(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Timeout(_) => StatusCode::GATEWAY_TIMEOUT,
            AppError::Io(_) | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = Json(json!({ "error": self.to_string() }));
        (status, body).into_response()
    }
}
