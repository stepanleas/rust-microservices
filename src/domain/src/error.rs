use shared::error::AppError;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("{message}")]
    OrderDomainError { message: String },

    #[error("{message}")]
    SagaError { message: String },
}

/// Lives here rather than in the application crate: the orphan rule only allows the impl in
/// the crate that owns `DomainError` (`shared::error` itself is framework free).
impl From<DomainError> for AppError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::OrderDomainError { .. } => AppError::BusinessRule(error.to_string()),
            DomainError::SagaError { .. } => AppError::Conflict(error.to_string()),
        }
    }
}
