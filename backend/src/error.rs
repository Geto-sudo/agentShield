use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// API error containing an HTTP status, a stable code, and an English message.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Denied(String),
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("{0}")]
    Internal(String),
    #[error("PayPal error: {0}")]
    PayPal(String),
}

impl ApiError {
    pub const fn code(&self) -> &'static str {
        match self {
            ApiError::NotFound(_) => "not_found",
            ApiError::BadRequest(_) => "bad_request",
            ApiError::Conflict(_) => "conflict",
            ApiError::Denied(_) => "guardrail_denied",
            ApiError::Db(_) => "database_error",
            ApiError::Internal(_) => "internal_error",
            ApiError::PayPal(_) => "paypal_error",
        }
    }

    pub const fn status(&self) -> StatusCode {
        match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Denied(_) => StatusCode::FORBIDDEN,
            ApiError::Db(_) | ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::PayPal(_) => StatusCode::BAD_GATEWAY,
        }
    }

    pub fn not_found(what: impl std::fmt::Display) -> Self {
        ApiError::NotFound(format!("{what} not found"))
    }

    pub fn bad_request(what: impl std::fmt::Display) -> Self {
        ApiError::BadRequest(what.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = Json(json!({
            "error": {
                "code": self.code(),
                "message": self.to_string(),
            }
        }));
        (status, body).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
