use domain::aggregates::payment::Payment;
use domain::enums::PaymentStatus;
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PaymentDto {
    id: Uuid,
    order_id: Uuid,
    customer_id: Uuid,
    price: Decimal,
    payment_status: PaymentStatus,
    failure_messages: Vec<String>,
}

impl PaymentDto {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn order_id(&self) -> Uuid {
        self.order_id
    }

    pub fn customer_id(&self) -> Uuid {
        self.customer_id
    }

    pub fn price(&self) -> Decimal {
        self.price
    }

    pub fn payment_status(&self) -> PaymentStatus {
        self.payment_status
    }

    pub fn payment_status_as_str(&self) -> &'static str {
        self.payment_status.as_str()
    }

    pub fn failure_messages(&self) -> &[String] {
        &self.failure_messages
    }
}

impl From<&Payment> for PaymentDto {
    fn from(payment: &Payment) -> Self {
        Self {
            id: payment.id().into(),
            order_id: payment.order_id().into(),
            customer_id: payment.customer_id().into(),
            price: payment.price(),
            payment_status: payment.payment_status(),
            failure_messages: payment.failure_messages().to_vec(),
        }
    }
}

impl From<Payment> for PaymentDto {
    fn from(payment: Payment) -> Self {
        Self::from(&payment)
    }
}
