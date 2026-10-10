use shared::error::AppError;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    /// The change conflicts with an existing payment (e.g. a second payment for one order).
    #[error("{message}")]
    Conflict { message: String },

    /// A payment business rule is violated.
    #[error("{message}")]
    PaymentDomainError { message: String },
}

// Lives here rather than in the application crate: neither `AppError` nor `DomainError` is
// local to the application crate, so the orphan rule only allows this impl in this crate.
impl From<DomainError> for AppError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::Conflict { message } => AppError::Conflict(message),
            DomainError::PaymentDomainError { message } => AppError::BusinessRule(message),
        }
    }
}
