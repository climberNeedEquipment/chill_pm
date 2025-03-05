use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    InternalError(String),
    NotFound(String),
    Unauthorized(String),
}

impl AppError {
    pub fn bad_request(message: String) -> Self {
        AppError::BadRequest(message)
    }
    pub fn internal_error(message: String) -> Self {
        AppError::InternalError(message)
    }
    pub fn not_found(message: String) -> Self {
        AppError::NotFound(message)
    }
    pub fn unauthorized(message: String) -> Self {
        AppError::Unauthorized(message)
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    status: String,
    message: String,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            AppError::BadRequest(msg) => msg,
            AppError::InternalError(msg) => msg,
            AppError::NotFound(msg) => msg,
            AppError::Unauthorized(msg) => msg,
        };
        write!(f, "{}", message)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
        };

        let body = Json(ErrorResponse {
            status: "error".to_string(),
            message,
        });

        (status, body).into_response()
    }
}

impl std::error::Error for AppError {}
