use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::fmt;
use tracing::error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    ExternalApi(String),
    Internal(String),
    Timeout(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotFound(msg) => {
                write!(f, "Not found: {}", msg)
            }
            AppError::ExternalApi(msg) => {
                write!(f, "External API error: {}", msg)
            }
            AppError::Internal(msg) => {
                write!(f, "Internal error: {}", msg)
            }
            AppError::Timeout(msg) => write!(f, "Timeout: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, msg.clone())
            }
            AppError::ExternalApi(msg) => {
                (StatusCode::BAD_GATEWAY, msg.clone())
            }
            AppError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
            AppError::Timeout(msg) => {
                (StatusCode::GATEWAY_TIMEOUT, msg.clone())
            }
        };

        // Log the error
        error!(error = %self, status_code = %status, "Request failed");

        let body = Json(ErrorResponse {
            error: error_message,
            details: None,
        });

        (status, body).into_response()
    }
}
