//! The saga messages the order service writes to its outbox, and their payloads.

use chrono::Utc;
use domain::aggregates::order::Order;
use serde::{Deserialize, Serialize};
use shared::outbox::{MessageKind, OutboxMessage};
use uuid::Uuid;

/// Kind of saga message stored in the outbox. Each kind is published to its own Kafka topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxMessageType {
    RestaurantApprovalRequest,
    PaymentRequest,
    /// Compensation: the payment failed after the restaurant approved (and reserved) the order.
    RestaurantApprovalCancelRequest,
}

impl MessageKind for OutboxMessageType {
    fn as_str(&self) -> &'static str {
        match self {
            OutboxMessageType::RestaurantApprovalRequest => "RESTAURANT_APPROVAL_REQUEST",
            OutboxMessageType::PaymentRequest => "PAYMENT_REQUEST",
            OutboxMessageType::RestaurantApprovalCancelRequest => {
                "RESTAURANT_APPROVAL_CANCEL_REQUEST"
            }
        }
    }

    fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "RESTAURANT_APPROVAL_REQUEST" => Ok(OutboxMessageType::RestaurantApprovalRequest),
            "PAYMENT_REQUEST" => Ok(OutboxMessageType::PaymentRequest),
            "RESTAURANT_APPROVAL_CANCEL_REQUEST" => {
                Ok(OutboxMessageType::RestaurantApprovalCancelRequest)
            }
            other => Err(anyhow::anyhow!("Unknown outbox message type: {other}")),
        }
    }
}

pub type OrderOutboxMessage = OutboxMessage<OutboxMessageType>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderApprovalEventProduct {
    pub id: Uuid,
    pub quantity: i32,
}

fn products_of(order: &Order) -> Vec<OrderApprovalEventProduct> {
    order
        .items()
        .iter()
        .map(|item| OrderApprovalEventProduct {
            id: *item.product_id().as_uuid(),
            quantity: item.quantity(),
        })
        .collect()
}

/// Payload of the `restaurant-approval-request` message sent to the restaurant service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderApprovalEventPayload {
    pub id: Uuid,
    pub saga_id: Uuid,
    pub order_id: Uuid,
    pub customer_id: Uuid,
    pub price: String,
    pub products: Vec<OrderApprovalEventProduct>,
    pub created_at: String,
}

impl OrderApprovalEventPayload {
    pub fn from_order(order: &Order, saga_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            saga_id,
            order_id: *order.id().as_uuid(),
            customer_id: *order.customer_id().as_uuid(),
            price: format!("{:.2}", order.price().amount()),
            products: products_of(order),
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Payload of the `restaurant-approval-cancel-request` message sent to the restaurant service
/// when an approved order is cancelled, so the restaurant gives the reserved stock back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderApprovalCancelEventPayload {
    pub id: Uuid,
    pub saga_id: Uuid,
    pub order_id: Uuid,
    pub products: Vec<OrderApprovalEventProduct>,
    pub created_at: String,
}

impl OrderApprovalCancelEventPayload {
    pub fn from_order(order: &Order, saga_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            saga_id,
            order_id: *order.id().as_uuid(),
            products: products_of(order),
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Payload of the `payment-request` message sent to the payment service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderPaymentEventPayload {
    pub id: Uuid,
    pub saga_id: Uuid,
    pub order_id: Uuid,
    pub customer_id: Uuid,
    pub price: String,
    pub created_at: String,
}

impl OrderPaymentEventPayload {
    pub fn from_order(order: &Order, saga_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            saga_id,
            order_id: *order.id().as_uuid(),
            customer_id: *order.customer_id().as_uuid(),
            price: format!("{:.2}", order.price().amount()),
            created_at: Utc::now().to_rfc3339(),
        }
    }
}
