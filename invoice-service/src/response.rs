use actix_web::HttpResponse;
use serde::Serialize;

/// API response status
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiStatus {
    Success,
    Error,
}

/// Generic response envelope for single items
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub status: ApiStatus,
    pub data: T,
}

/// Response envelope for errors
#[derive(Debug, Serialize)]
pub struct ApiErrorResponse {
    pub status: ApiStatus,
    pub error: ErrorDetails,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetails {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a success response with data
    pub fn success(data: T) -> Self {
        Self {
            status: ApiStatus::Success,
            data,
        }
    }

    /// Convert to HttpResponse with custom status code
    pub fn with_status(self, status: actix_web::http::StatusCode) -> HttpResponse {
        HttpResponse::build(status).json(self)
    }

    /// Convert to Created (201) response
    pub fn created(self) -> HttpResponse {
        self.with_status(actix_web::http::StatusCode::CREATED)
    }

    /// Convert to Ok (200) response
    pub fn ok(self) -> HttpResponse {
        self.with_status(actix_web::http::StatusCode::OK)
    }

    /// Convert to Accepted (202) response
    pub fn accepted(self) -> HttpResponse {
        self.with_status(actix_web::http::StatusCode::ACCEPTED)
    }
}

impl ApiErrorResponse {
    /// Create an error response
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: ApiStatus::Error,
            error: ErrorDetails {
                code: code.into(),
                message: message.into(),
                details: None,
            },
        }
    }

    /// Create an error response with details
    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            status: ApiStatus::Error,
            error: ErrorDetails {
                code: code.into(),
                message: message.into(),
                details: Some(details),
            },
        }
    }

    /// Convert to HttpResponse with custom status code
    pub fn with_status(self, status: actix_web::http::StatusCode) -> HttpResponse {
        HttpResponse::build(status).json(self)
    }
}

/// Helper struct for list responses
#[derive(Debug, Serialize)]
pub struct ListData<T> {
    pub items: Vec<T>,
    pub total: usize,
}

impl<T: Serialize> ListData<T> {
    pub fn new(items: Vec<T>, total: usize) -> Self {
        Self { items, total }
    }
}
