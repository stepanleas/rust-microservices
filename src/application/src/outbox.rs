use chrono::SecondsFormat;
use domain::aggregates::order_approval::OrderApproval;
use serde::{Deserialize, Serialize};
use shared::outbox::{MessageKind, OutboxMessage};
use uuid::Uuid;

/// The messages the restaurant service writes to its outbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxMessageType {
    RestaurantApprovalResponse,
}

impl MessageKind for OutboxMessageType {
    fn as_str(&self) -> &'static str {
        match self {
            OutboxMessageType::RestaurantApprovalResponse => "RESTAURANT_APPROVAL_RESPONSE",
        }
    }

    fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "RESTAURANT_APPROVAL_RESPONSE" => Ok(OutboxMessageType::RestaurantApprovalResponse),
            other => Err(anyhow::anyhow!("Unknown outbox message type: {other}")),
        }
    }
}

pub type RestaurantOutboxMessage = OutboxMessage<OutboxMessageType>;

/// JSON payload stored in the outbox for a `restaurant-approval-response` message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestaurantApprovalResponsePayload {
    pub saga_id: Uuid,
    pub order_id: Uuid,
    pub order_approval_status: String,
    pub failure_messages: Vec<String>,
    pub created_at: String,
}

impl From<&OrderApproval> for RestaurantApprovalResponsePayload {
    fn from(approval: &OrderApproval) -> Self {
        Self {
            saga_id: approval.saga_id(),
            order_id: *approval.order_id().as_uuid(),
            order_approval_status: approval.status().as_str().to_string(),
            failure_messages: approval.failure_messages().to_vec(),
            created_at: approval
                .created_at()
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

/// The outbox message answering the order service with the restaurant's decision.
pub fn restaurant_approval_response(
    approval: &OrderApproval,
) -> anyhow::Result<RestaurantOutboxMessage> {
    OutboxMessage::new(
        approval.saga_id(),
        OutboxMessageType::RestaurantApprovalResponse,
        &RestaurantApprovalResponsePayload::from(approval),
    )
}
