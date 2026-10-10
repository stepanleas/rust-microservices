use super::entity::{Payment, PaymentId};
use crate::enums::PaymentStatus;
use rust_decimal::Decimal;
use shared::domain::value_objects::{CustomerId, OrderId};

#[derive(Default)]
pub struct PaymentBuilder {
    id: PaymentId,
    order_id: OrderId,
    customer_id: CustomerId,
    price: Decimal,
    payment_status: PaymentStatus,
    failure_messages: Vec<String>,
}

impl PaymentBuilder {
    pub fn id(mut self, id: PaymentId) -> Self {
        self.id = id;
        self
    }

    pub fn order_id(mut self, order_id: OrderId) -> Self {
        self.order_id = order_id;
        self
    }

    pub fn customer_id(mut self, customer_id: CustomerId) -> Self {
        self.customer_id = customer_id;
        self
    }

    pub fn price(mut self, price: Decimal) -> Self {
        self.price = price;
        self
    }

    pub fn payment_status(mut self, payment_status: PaymentStatus) -> Self {
        self.payment_status = payment_status;
        self
    }

    pub fn failure_messages(mut self, failure_messages: Vec<String>) -> Self {
        self.failure_messages = failure_messages;
        self
    }

    pub fn build(self) -> Payment {
        let mut payment = Payment::new(
            self.id,
            self.order_id,
            self.customer_id,
            self.price,
            self.payment_status,
        );
        payment.set_failure_messages(self.failure_messages);

        payment
    }
}
