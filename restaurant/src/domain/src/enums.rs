use crate::error::DomainError;
use std::fmt::Display;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum OrderApprovalStatus {
    Approved,
    Rejected,
    /// An approved order was cancelled later in the saga (compensation) and its stock given back.
    Cancelled,
}

impl OrderApprovalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderApprovalStatus::Approved => "APPROVED",
            OrderApprovalStatus::Rejected => "REJECTED",
            OrderApprovalStatus::Cancelled => "CANCELLED",
        }
    }
}

impl TryFrom<&str> for OrderApprovalStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "APPROVED" => Ok(OrderApprovalStatus::Approved),
            "REJECTED" => Ok(OrderApprovalStatus::Rejected),
            "CANCELLED" => Ok(OrderApprovalStatus::Cancelled),
            other => Err(DomainError::RestaurantDomainError {
                message: format!("Unknown order approval status: {other}"),
            }),
        }
    }
}

impl Display for OrderApprovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
