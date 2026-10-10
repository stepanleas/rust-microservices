use crate::aggregates::order::Order;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct OrderCreatedEvent {
    id: Uuid,
    order: Order,
    created_at: DateTime<Utc>,
}

impl OrderCreatedEvent {
    pub fn new(order: Order) -> Self {
        Self {
            id: Uuid::new_v4(),
            order,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn order(&self) -> &Order {
        &self.order
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

pub struct OrderPaidEvent {
    id: Uuid,
    order: Order,
    created_at: DateTime<Utc>,
}

impl OrderPaidEvent {
    pub fn new(order: Order) -> Self {
        Self {
            id: Uuid::new_v4(),
            order,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn order(&self) -> &Order {
        &self.order
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

pub struct OrderCancelledEvent {
    id: Uuid,
    order: Order,
    created_at: DateTime<Utc>,
}

impl OrderCancelledEvent {
    pub fn new(order: Order) -> Self {
        Self {
            id: Uuid::new_v4(),
            order,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn order(&self) -> &Order {
        &self.order
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
