use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

use crate::response::ApiErrorResponse;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound(String),
    #[error("bad request")]
    BadRequest(String),
    #[error("conflict")]
    Conflict(String),
    #[error("internal")]
    Internal(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unauthorized => "UNAUTHORIZED",
            Self::NotFound(_) => "NOT_FOUND",
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::Conflict(_) => "CONFLICT",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Unauthorized => "Invalid or missing API key".to_string(),
            Self::NotFound(msg)
            | Self::BadRequest(msg)
            | Self::Conflict(msg)
            | Self::Internal(msg) => msg.clone(),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        ApiErrorResponse::error(self.code(), self.message())
            .with_status(self.status_code())
    }
}

impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self {
        Self::Internal(format!("database error: {value}"))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        Self::Internal(format!("request error: {value}"))
    }
}
