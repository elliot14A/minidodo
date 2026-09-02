mod error_codes;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

pub use error_codes::*;

#[derive(Debug, Error, Clone)]
pub enum MinidodoError {
    #[error("Bad request: {message}")]
    BadRequest { message: String, code: &'static str },

    #[error("Unauthorized: {message}")]
    Unauthorized { message: String, code: &'static str },

    #[error("Forbidden: {message}")]
    Forbidden { message: String, code: &'static str },

    #[error("{details}")]
    NotFound { details: String, code: &'static str },

    #[error("Conflict: {message}")]
    Conflict { message: String, code: &'static str },

    #[error("Duplicate resource: {details}")]
    Duplicate { details: String, code: &'static str },

    #[error("Unprocessable entity: {message}")]
    UnprocessableEntity { message: String, code: &'static str },

    #[error("Constraint violation on {resource}: {details}")]
    ConstraintViolation {
        resource: String,
        details: String,
        code: &'static str,
    },

    #[error("Database connection error: {message}")]
    DatabaseConnection { message: String, code: &'static str },

    #[error("Database error: {message}")]
    DatabaseError { message: String, code: &'static str },

    #[error("Internal error: {message}")]
    Internal { message: String, code: &'static str },

    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String, code: &'static str },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationErrorDetail {
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub message: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ValidationErrorDetail>>,
}

impl IntoResponse for MinidodoError {
    fn into_response(self) -> Response {
        let (status, message, code) = match &self {
            MinidodoError::BadRequest { message, code } => {
                (StatusCode::BAD_REQUEST, message.clone(), *code)
            }
            MinidodoError::Unauthorized { message, code } => {
                (StatusCode::UNAUTHORIZED, message.clone(), *code)
            }
            MinidodoError::Forbidden { message, code } => {
                (StatusCode::FORBIDDEN, message.clone(), *code)
            }
            MinidodoError::NotFound { details, code } => {
                (StatusCode::NOT_FOUND, details.clone(), *code)
            }
            MinidodoError::Conflict { message, code } => {
                (StatusCode::CONFLICT, message.clone(), *code)
            }
            MinidodoError::Duplicate { details, code } => {
                (StatusCode::CONFLICT, details.clone(), *code)
            }
            MinidodoError::UnprocessableEntity { message, code } => {
                (StatusCode::UNPROCESSABLE_ENTITY, message.clone(), *code)
            }
            MinidodoError::ConstraintViolation {
                resource,
                details,
                code,
            } => (
                StatusCode::PRECONDITION_FAILED,
                format!("Constraint violation on {}: {}", resource, details),
                *code,
            ),
            MinidodoError::DatabaseConnection { code, .. } => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Failed to communicate with database. Please try again later.".to_string(),
                *code,
            ),
            MinidodoError::DatabaseError { code, .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A database error occurred. Please try again later.".to_string(),
                *code,
            ),
            MinidodoError::ServiceUnavailable { message, code } => {
                (StatusCode::SERVICE_UNAVAILABLE, message.clone(), *code)
            }
            MinidodoError::Internal { code, .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unexpected internal error occurred.".to_string(),
                *code,
            ),
        };

        let body = Json(ErrorResponse {
            message,
            code: code.to_string(),
            errors: None,
        });

        (status, body).into_response()
    }
}

pub type Result<T, E = MinidodoError> = std::result::Result<T, E>;
