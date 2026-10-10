use super::ValidationFieldError;
use crate::error::AppError;
use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::ValidationErrors;

/// Body of every error response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({ "code": 404, "message": "Could not find resource with id: 7c9e6679-7425-40de-944b-e07fc1f90ae7" }))]
pub struct ErrorResponse {
    /// HTTP status code.
    pub code: u16,
    /// Human readable error description.
    pub message: String,
    /// Field level errors, only present for validation errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ValidationFieldError>>,
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ApiError {
    code: StatusCode,
    message: String,
    errors: Option<Vec<ValidationFieldError>>,
}

impl ApiError {
    pub fn new(code: StatusCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            errors: None,
        }
    }

    /// Logs the cause; the caller only sees a generic message.
    pub fn internal(error: impl std::fmt::Display) -> Self {
        tracing::error!("Internal server error: {error}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }
}

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        match error {
            AppError::NotFound(message) => Self::new(StatusCode::NOT_FOUND, message),
            AppError::Validation(message) => Self::new(StatusCode::BAD_REQUEST, message),
            AppError::BusinessRule(message) => Self::new(StatusCode::UNPROCESSABLE_ENTITY, message),
            AppError::Conflict(message) => Self::new(StatusCode::CONFLICT, message),
            AppError::Internal(error) => Self::internal(format!("{error:#}")),
        }
    }
}

impl From<ValidationErrors> for ApiError {
    fn from(errors: ValidationErrors) -> Self {
        let mut result = Vec::new();

        for (field, validation_errors) in errors.field_errors().iter() {
            for validation_error in validation_errors.iter() {
                result.push(ValidationFieldError {
                    field: field.to_string(),
                    message: validation_error
                        .message
                        .as_deref()
                        .unwrap_or("Invalid value")
                        .to_string(),
                });
            }
        }

        Self {
            code: StatusCode::BAD_REQUEST,
            message: "Validation error".to_string(),
            errors: Some(result),
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.code
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.code).json(ErrorResponse {
            code: self.code.as_u16(),
            message: self.message.clone(),
            errors: self.errors.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_app_errors_to_status_codes() {
        let cases = [
            (AppError::not_found("x"), StatusCode::NOT_FOUND),
            (AppError::validation("x"), StatusCode::BAD_REQUEST),
            (
                AppError::business_rule("x"),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (AppError::conflict("x"), StatusCode::CONFLICT),
            (
                AppError::Internal(anyhow::anyhow!("db down")),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        for (error, status) in cases {
            assert_eq!(status, ApiError::from(error).status_code());
        }
    }

    #[test]
    fn internal_errors_do_not_leak_details() {
        let error = ApiError::from(AppError::Internal(anyhow::anyhow!("password=secret")));
        assert_eq!("Internal server error", error.to_string());
    }
}
