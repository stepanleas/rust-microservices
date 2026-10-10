use std::fmt;

/// Outcome of a payment.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub enum PaymentStatus {
    #[default]
    Completed,
    Cancelled,
    Failed,
}

impl PaymentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentStatus::Completed => "COMPLETED",
            PaymentStatus::Cancelled => "CANCELLED",
            PaymentStatus::Failed => "FAILED",
        }
    }

    /// Parses a status name, ignoring case. Returns `None` for an unknown status.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_uppercase().as_str() {
            "COMPLETED" => Some(PaymentStatus::Completed),
            "CANCELLED" => Some(PaymentStatus::Cancelled),
            "FAILED" => Some(PaymentStatus::Failed),
            _ => None,
        }
    }
}

impl fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
