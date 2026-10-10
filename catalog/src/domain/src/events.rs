use crate::aggregates::product::Product;
use chrono::{DateTime, Utc};
use shared::domain::value_objects::ProductId;
use uuid::Uuid;

#[derive(Debug, PartialEq)]
pub struct ProductCreatedEvent {
    id: Uuid,
    product: Product,
    created_at: DateTime<Utc>,
}

impl ProductCreatedEvent {
    pub fn new(product: Product) -> Self {
        Self {
            id: Uuid::new_v4(),
            product,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn product(&self) -> &Product {
        &self.product
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

pub struct ProductUpdatedEvent {
    id: Uuid,
    product: Product,
    created_at: DateTime<Utc>,
}

impl ProductUpdatedEvent {
    pub fn new(product: Product) -> Self {
        Self {
            id: Uuid::new_v4(),
            product,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn product(&self) -> &Product {
        &self.product
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

pub struct ProductDeletedEvent {
    id: Uuid,
    product_id: ProductId,
    created_at: DateTime<Utc>,
}

impl ProductDeletedEvent {
    pub fn new(product_id: ProductId) -> Self {
        Self {
            id: Uuid::new_v4(),
            product_id,
            created_at: Utc::now(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn product_id(&self) -> ProductId {
        self.product_id
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
