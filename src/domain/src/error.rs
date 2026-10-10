use crate::enums::OrderApprovalStatus;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("{message}")]
    RestaurantDomainError { message: String },

    #[error("Order approval of order {order_id} cannot be cancelled in status {status}")]
    ApprovalNotCancellable {
        order_id: String,
        status: OrderApprovalStatus,
    },
}

/// How the application layer reports a broken domain rule. Lives here rather than in the
/// application crate because the orphan rule forbids implementing a foreign trait (`From`)
/// between two foreign types there.
impl From<DomainError> for shared::error::AppError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::RestaurantDomainError { .. } => Self::business_rule(error.to_string()),
            DomainError::ApprovalNotCancellable { .. } => Self::conflict(error.to_string()),
        }
    }
}
