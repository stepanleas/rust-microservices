use thiserror::Error;

/// Error returned by the application layer (use cases) of every service.
///
/// Each variant says how the caller should react, so the presentation layer can map it to an
/// HTTP status without knowing anything about the domain: a service converts its own
/// `DomainError` into one of these variants with a `From` impl in its application crate.
#[derive(Debug, Error)]
pub enum AppError {
    /// The requested resource does not exist (HTTP 404).
    #[error("{0}")]
    NotFound(String),

    /// The input is malformed (HTTP 400).
    #[error("{0}")]
    Validation(String),

    /// The request is well formed but breaks a business rule (HTTP 422).
    #[error("{0}")]
    BusinessRule(String),

    /// The request conflicts with the current state, e.g. a saga already moved on (HTTP 409).
    #[error("{0}")]
    Conflict(String),

    /// Anything the caller cannot fix: database down, broker unreachable, bug (HTTP 500).
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn business_rule(message: impl Into<String>) -> Self {
        Self::BusinessRule(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }
}
