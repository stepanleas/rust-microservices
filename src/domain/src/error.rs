use shared::error::AppError;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("invalid money value")]
    InvalidMoneyValue,
}

// Lives here rather than in the application crate: the orphan rule only allows this impl in the
// crate that owns `DomainError` (both `From` and `AppError` are foreign everywhere else).
impl From<DomainError> for AppError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::InvalidMoneyValue => AppError::validation(error.to_string()),
        }
    }
}
