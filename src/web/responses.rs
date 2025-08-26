use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub meta: Option<Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            meta: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn success_with_meta(data: T, meta: Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            meta: Some(meta),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: message.into(),
                details: None,
            }),
            meta: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error_with_details(
        code: impl Into<String>, 
        message: impl Into<String>, 
        details: Value
    ) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: message.into(),
                details: Some(details),
            }),
            meta: None,
            timestamp: chrono::Utc::now(),
        }
    }
}

// Custom error types for the API
#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized,
    Forbidden,
    NotFound(String),
    Conflict(String),
    UnprocessableEntity(String),
    InternalServerError(String),
    ServiceUnavailable(String),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::Unauthorized => "UNAUTHORIZED",
            AppError::Forbidden => "FORBIDDEN",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Conflict(_) => "CONFLICT",
            AppError::UnprocessableEntity(_) => "UNPROCESSABLE_ENTITY",
            AppError::InternalServerError(_) => "INTERNAL_SERVER_ERROR",
            AppError::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
        }
    }

    pub fn message(&self) -> String {
        match self {
            AppError::BadRequest(msg) => msg.clone(),
            AppError::Unauthorized => "Authentication required".to_string(),
            AppError::Forbidden => "Access denied".to_string(),
            AppError::NotFound(msg) => msg.clone(),
            AppError::Conflict(msg) => msg.clone(),
            AppError::UnprocessableEntity(msg) => msg.clone(),
            AppError::InternalServerError(msg) => msg.clone(),
            AppError::ServiceUnavailable(msg) => msg.clone(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ApiResponse::<()>::error(self.error_code(), self.message());
        (status, Json(body)).into_response()
    }
}

// Common error constructors
impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound(format!("{} not found", resource.into()))
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }

    pub fn unprocessable(msg: impl Into<String>) -> Self {
        Self::UnprocessableEntity(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::InternalServerError(msg.into())
    }

    pub fn unavailable(msg: impl Into<String>) -> Self {
        Self::ServiceUnavailable(msg.into())
    }
}

// Pagination response helper
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub page: u32,
    pub per_page: u32,
    pub total: u32,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, page: u32, per_page: u32, total: u32) -> Self {
        let total_pages = (total as f32 / per_page as f32).ceil() as u32;
        
        Self {
            items,
            pagination: PaginationMeta {
                page,
                per_page,
                total,
                total_pages,
                has_next: page < total_pages,
                has_prev: page > 1,
            },
        }
    }
}

// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub uptime: String,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
    pub duration_ms: Option<u64>,
}

impl HealthResponse {
    pub fn healthy() -> Self {
        Self {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: "calculated from start time".to_string(), // In a real implementation, calculate actual uptime
            checks: vec![
                HealthCheck {
                    name: "database".to_string(),
                    status: "healthy".to_string(),
                    message: Some("Database connection is active".to_string()),
                    duration_ms: Some(1),
                },
                HealthCheck {
                    name: "scheduler".to_string(),
                    status: "healthy".to_string(),
                    message: Some("Scheduler is running".to_string()),
                    duration_ms: Some(1),
                },
            ],
        }
    }

    pub fn unhealthy(checks: Vec<HealthCheck>) -> Self {
        Self {
            status: "unhealthy".to_string(),
            timestamp: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: "calculated from start time".to_string(),
            checks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("test data");
        assert!(response.success);
        assert_eq!(response.data, Some("test data"));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_success_with_meta() {
        let meta = serde_json::json!({"key": "value"});
        let response = ApiResponse::success_with_meta("test data", meta.clone());
        assert!(response.success);
        assert_eq!(response.data, Some("test data"));
        assert_eq!(response.meta, Some(meta));
    }

    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::<()>::error("TEST_ERROR", "Test error message");
        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(response.error.is_some());
        
        let error = response.error.unwrap();
        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "Test error message");
    }

    #[test]
    fn test_app_error_status_codes() {
        assert_eq!(AppError::BadRequest("msg".to_string()).status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(AppError::Unauthorized.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(AppError::NotFound("resource".to_string()).status_code(), StatusCode::NOT_FOUND);
        assert_eq!(AppError::InternalServerError("msg".to_string()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_app_error_codes() {
        assert_eq!(AppError::BadRequest("msg".to_string()).error_code(), "BAD_REQUEST");
        assert_eq!(AppError::Unauthorized.error_code(), "UNAUTHORIZED");
        assert_eq!(AppError::NotFound("resource".to_string()).error_code(), "NOT_FOUND");
    }

    #[test]
    fn test_app_error_constructors() {
        let bad_request = AppError::bad_request("Invalid input");
        assert!(matches!(bad_request, AppError::BadRequest(_)));
        
        let not_found = AppError::not_found("Product");
        assert!(matches!(not_found, AppError::NotFound(_)));
        assert_eq!(not_found.message(), "Product not found");
    }

    #[test]
    fn test_paginated_response() {
        let items = vec!["item1", "item2", "item3"];
        let response = PaginatedResponse::new(items.clone(), 1, 2, 10);
        
        assert_eq!(response.items, items);
        assert_eq!(response.pagination.page, 1);
        assert_eq!(response.pagination.per_page, 2);
        assert_eq!(response.pagination.total, 10);
        assert_eq!(response.pagination.total_pages, 5);
        assert!(response.pagination.has_next);
        assert!(!response.pagination.has_prev);
    }

    #[test]
    fn test_health_response() {
        let health = HealthResponse::healthy();
        assert_eq!(health.status, "healthy");
        assert!(!health.checks.is_empty());
        assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
    }
}