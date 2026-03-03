use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Unified API error type for the daemon.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("capability denied: {0}")]
    CapabilityDenied(String),

    #[error("invalid state transition: {0}")]
    InvalidTransition(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    code: u16,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            ApiError::CapabilityDenied(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            ApiError::InvalidTransition(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
        };

        let body = ErrorBody {
            error: message,
            code: status.as_u16(),
        };

        (status, axum::Json(body)).into_response()
    }
}
