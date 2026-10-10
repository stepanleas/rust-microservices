use domain::aggregates::payment::Payment;
use serde::{Deserialize, Serialize};
use shared::outbox::{MessageKind, OutboxMessage};
use uuid::Uuid;

/// The kinds of messages this service writes to its outbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxMessageType {
    PaymentResponse,
}

impl MessageKind for OutboxMessageType {
    fn as_str(&self) -> &'static str {
        match self {
            OutboxMessageType::PaymentResponse => "PAYMENT_RESPONSE",
        }
    }

    fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "PAYMENT_RESPONSE" => Ok(OutboxMessageType::PaymentResponse),
            other => Err(anyhow::anyhow!("Unknown outbox message type: {other}")),
        }
    }
}

pub type PaymentOutboxMessage = OutboxMessage<OutboxMessageType>;

/// JSON payload stored in the outbox for a `PAYMENT_RESPONSE` message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentResponsePayload {
    pub saga_id: Uuid,
    pub payment_id: Uuid,
    pub order_id: Uuid,
    pub customer_id: Uuid,
    pub price: String,
    pub payment_status: String,
    pub failure_messages: Vec<String>,
    pub created_at: String,
}

impl PaymentResponsePayload {
    pub fn from_payment(saga_id: Uuid, payment: &Payment) -> Self {
        Self {
            saga_id,
            payment_id: payment.id().into(),
            order_id: payment.order_id().into(),
            customer_id: payment.customer_id().into(),
            price: payment.price().to_string(),
            payment_status: payment.payment_status().as_str().to_string(),
            failure_messages: payment.failure_messages().to_vec(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
