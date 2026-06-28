use axum::{Json, http::StatusCode, response::IntoResponse};
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

    /// Saturated concurrency slots within --queue-timeout-secs.
    /// Distinct from `Timeout` (504, single-query plocate killed) — this is
    /// admission control: the server is over-subscribed and refuses new work
    /// so clients can retry / fail fast instead of waiting silently.
    #[error("queue timeout: {0}")]
    QueueTimeout(String),

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
            AppError::QueueTimeout(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Timeout(_) => StatusCode::GATEWAY_TIMEOUT,
            AppError::Io(_) | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = Json(json!({ "error": self.to_string() }));
        (status, body).into_response()
    }
}
