use crate::aggregates::customer::Customer;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct CustomerCreatedEvent {
    id: Uuid,
    customer: Customer,
    created_at: DateTime<Utc>,
}

impl CustomerCreatedEvent {
    pub fn new(customer: Customer) -> Self {
        Self {
            id: Uuid::new_v4(),
            customer,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn customer(&self) -> &Customer {
        &self.customer
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

pub struct CustomerUpdatedEvent {
    id: Uuid,
    customer: Customer,
    created_at: DateTime<Utc>,
}

impl CustomerUpdatedEvent {
    pub fn new(customer: Customer) -> Self {
        Self {
            id: Uuid::new_v4(),
            customer,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn customer(&self) -> &Customer {
        &self.customer
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
