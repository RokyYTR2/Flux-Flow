use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::fmt::{Display, Formatter};
use tracing::{error, warn};

use crate::models::ErrorResponse;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    TooManyRequests(String),
    Internal(String),
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            ApiError::BadRequest(message)
            | ApiError::Unauthorized(message)
            | ApiError::Forbidden(message)
            | ApiError::NotFound(message)
            | ApiError::TooManyRequests(message)
            | ApiError::Internal(message) => message,
        }
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let message = self.message().to_string();

        match status {
            StatusCode::BAD_REQUEST => warn!(error = %message, "Request validation failed"),
            StatusCode::UNAUTHORIZED => warn!(error = %message, "Authentication failed"),
            StatusCode::FORBIDDEN => warn!(error = %message, "Permission denied"),
            StatusCode::NOT_FOUND => warn!(error = %message, "Resource not found"),
            StatusCode::TOO_MANY_REQUESTS => warn!(error = %message, "Rate limited"),
            _ => error!(error = %message, "Unhandled backend error"),
        }

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}