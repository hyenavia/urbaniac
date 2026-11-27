use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(&'static str, String),
    Unauthorized(&'static str, String),
    NotFound(&'static str, String),
    Conflict(&'static str, String),
    TooManyRequests(&'static str, String),
    Internal(anyhow::Error),
}

impl ApiError {
    pub fn bad_request(code: &'static str, msg: impl Into<String>) -> Self {
        Self::BadRequest(code, msg.into())
    }

    pub fn unauthorized(code: &'static str, msg: impl Into<String>) -> Self {
        Self::Unauthorized(code, msg.into())
    }

    pub fn not_found(code: &'static str, msg: impl Into<String>) -> Self {
        Self::NotFound(code, msg.into())
    }

    pub fn conflict(code: &'static str, msg: impl Into<String>) -> Self {
        Self::Conflict(code, msg.into())
    }

    pub fn too_many_requests(code: &'static str, msg: impl Into<String>) -> Self {
        Self::TooManyRequests(code, msg.into())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err)
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        Self::Internal(err.into())
    }
}

#[derive(Serialize)]
struct ErrorPayload<'a> {
    error: ErrorDetail<'a>,
}

#[derive(Serialize)]
struct ErrorDetail<'a> {
    code: &'a str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(code, msg) => (StatusCode::BAD_REQUEST, code, msg),
            Self::Unauthorized(code, msg) => (StatusCode::UNAUTHORIZED, code, msg),
            Self::NotFound(code, msg) => (StatusCode::NOT_FOUND, code, msg),
            Self::Conflict(code, msg) => (StatusCode::CONFLICT, code, msg),
            Self::TooManyRequests(code, msg) => (StatusCode::TOO_MANY_REQUESTS, code, msg),
            Self::Internal(err) => {
                tracing::error!(error = ?err, "internal web API error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Internal server error".to_string(),
                )
            }
        };

        (status, Json(ErrorPayload { error: ErrorDetail { code, message } })).into_response()
    }
}
