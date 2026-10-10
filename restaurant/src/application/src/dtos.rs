use chrono::{DateTime, Utc};
use domain::aggregates::order_approval::OrderApproval;
use domain::aggregates::product::Product;
use domain::enums::OrderApprovalStatus;
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct OrderApprovalDto {
    pub id: Uuid,
    pub saga_id: Uuid,
    pub order_id: Uuid,
    pub status: OrderApprovalStatus,
    pub failure_messages: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl From<&OrderApproval> for OrderApprovalDto {
    fn from(approval: &OrderApproval) -> Self {
        Self {
            id: approval.id().into(),
            saga_id: approval.saga_id(),
            order_id: approval.order_id().into(),
            status: approval.status(),
            failure_messages: approval.failure_messages().to_vec(),
            created_at: approval.created_at(),
        }
    }
}

/// A product as replicated from the catalog, or as returned by the queries.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductDto {
    pub id: Uuid,
    pub title: String,
    pub quantity: i32,
    pub price: Decimal,
}

impl From<&Product> for ProductDto {
    fn from(product: &Product) -> Self {
        Self {
            id: product.id().into(),
            title: product.title().to_string(),
            quantity: product.quantity(),
            price: product.price(),
        }
    }
}

impl From<ProductDto> for Product {
    fn from(dto: ProductDto) -> Self {
        Product::builder()
            .id(shared::domain::value_objects::ProductId::from_uuid(dto.id))
            .title(dto.title)
            .quantity(dto.quantity)
            .price(dto.price)
            .build()
    }
}
