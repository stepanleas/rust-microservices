use super::builder::PaymentBuilder;
use crate::enums::PaymentStatus;
use rust_decimal::Decimal;
use shared::domain::value_objects::{CustomerId, OrderId};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct PaymentId(Uuid);

impl PaymentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl From<PaymentId> for Uuid {
    fn from(value: PaymentId) -> Self {
        value.0
    }
}

impl fmt::Display for PaymentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug)]
pub struct Payment {
    id: PaymentId,
    order_id: OrderId,
    customer_id: CustomerId,
    price: Decimal,
    payment_status: PaymentStatus,
    failure_messages: Vec<String>,
}

impl Payment {
    pub fn builder() -> PaymentBuilder {
        PaymentBuilder::default()
    }

    pub fn new(
        id: PaymentId,
        order_id: OrderId,
        customer_id: CustomerId,
        price: Decimal,
        payment_status: PaymentStatus,
    ) -> Self {
        Self {
            id,
            order_id,
            customer_id,
            price,
            payment_status,
            failure_messages: vec![],
        }
    }

    pub fn id(&self) -> PaymentId {
        self.id
    }

    pub fn order_id(&self) -> OrderId {
        self.order_id
    }

    pub fn customer_id(&self) -> CustomerId {
        self.customer_id
    }

    pub fn price(&self) -> Decimal {
        self.price
    }

    pub fn payment_status(&self) -> PaymentStatus {
        self.payment_status
    }

    pub fn failure_messages(&self) -> &[String] {
        &self.failure_messages
    }

    pub fn set_failure_messages(&mut self, failure_messages: Vec<String>) {
        self.failure_messages = failure_messages;
    }

    /// Simulates charging the customer. There is no real payment provider:
    /// a payment with a positive price is completed, anything else fails.
    pub fn process(&mut self) {
        if self.price <= Decimal::ZERO {
            self.payment_status = PaymentStatus::Failed;
            self.failure_messages = vec!["Total price must be greater than zero!".to_string()];
            return;
        }

        self.payment_status = PaymentStatus::Completed;
        self.failure_messages = vec![];
    }

    pub fn is_completed(&self) -> bool {
        self.payment_status == PaymentStatus::Completed
    }
}
