use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderApprovalStatus {
    Approved,
    Rejected,
}

impl TryFrom<&str> for OrderApprovalStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "APPROVED" => Ok(OrderApprovalStatus::Approved),
            "REJECTED" => Ok(OrderApprovalStatus::Rejected),
            other => Err(anyhow::anyhow!("Unknown order approval status: {other}")),
        }
    }
}

/// Reply of the restaurant service to a restaurant approval request.
#[derive(Debug, Clone, PartialEq)]
pub struct RestaurantApprovalResponse {
    pub saga_id: Uuid,
    pub order_id: Uuid,
    pub status: OrderApprovalStatus,
    pub failure_messages: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentResponseStatus {
    Completed,
    Failed,
    Cancelled,
}

impl TryFrom<&str> for PaymentResponseStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "COMPLETED" => Ok(PaymentResponseStatus::Completed),
            "FAILED" => Ok(PaymentResponseStatus::Failed),
            "CANCELLED" => Ok(PaymentResponseStatus::Cancelled),
            other => Err(anyhow::anyhow!("Unknown payment status: {other}")),
        }
    }
}

/// Reply of the payment service to a payment request.
#[derive(Debug, Clone, PartialEq)]
pub struct PaymentResponse {
    pub saga_id: Uuid,
    pub order_id: Uuid,
    pub payment_id: Uuid,
    pub status: PaymentResponseStatus,
    pub failure_messages: Vec<String>,
}
