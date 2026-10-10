use crate::aggregates::payment::Payment;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct PaymentCreatedEvent {
    id: Uuid,
    payment: Payment,
    created_at: DateTime<Utc>,
}

impl PaymentCreatedEvent {
    pub fn new(payment: Payment) -> Self {
        Self {
            id: Uuid::new_v4(),
            payment,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn payment(&self) -> &Payment {
        &self.payment
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

pub struct PaymentUpdatedEvent {
    id: Uuid,
    payment: Payment,
    created_at: DateTime<Utc>,
}

impl PaymentUpdatedEvent {
    pub fn new(payment: Payment) -> Self {
        Self {
            id: Uuid::new_v4(),
            payment,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn payment(&self) -> &Payment {
        &self.payment
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
