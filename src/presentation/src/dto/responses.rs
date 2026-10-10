use application::dtos::{OrderApprovalDto, ProductDto};
use chrono::{DateTime, Utc};
use domain::enums::OrderApprovalStatus;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

const OK_STATUS: &str = "Ok";

#[readonly::make]
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AppInfoResponse {
    /// Environment the service runs in.
    #[schema(example = "development")]
    environment: String,
}

impl AppInfoResponse {
    pub fn new(environment: &str) -> Self {
        Self {
            environment: environment.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AppInfoDataResponse {
    pub data: AppInfoResponse,
}

#[readonly::make]
#[derive(Default, Serialize, Deserialize, ToSchema)]
pub struct HealthCheckResponse {
    #[schema(example = "Ok")]
    pub status: String,
}

impl HealthCheckResponse {
    pub fn new() -> Self {
        Self {
            status: OK_STATUS.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct HealthCheckDataResponse {
    pub data: HealthCheckResponse,
}

/// Restaurant decision for an order, produced while handling the order saga.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderApprovalResponse {
    /// Identifier of the approval record.
    #[schema(example = "3f1e6a52-0c1d-4f4a-9a0e-0b5e7c1d2a33")]
    pub id: Uuid,
    /// Saga the approval belongs to.
    #[schema(example = "5b0c50f1-7ab4-4d0b-9a5d-5f6a2f0e8b11")]
    pub saga_id: Uuid,
    /// Order that was approved or rejected.
    #[schema(example = "8f0b3a52-5d1e-4c39-8f4d-3c1d2e1f0a22")]
    pub order_id: Uuid,
    /// Decision of the restaurant.
    pub status: OrderApprovalStatusResponse,
    /// Reasons for a rejection; empty when approved.
    #[schema(example = json!(["Insufficient stock for product Pizza: requested 3, available 2"]))]
    pub failure_messages: Vec<String>,
    /// When the decision was taken.
    pub created_at: DateTime<Utc>,
}

impl From<OrderApprovalDto> for OrderApprovalResponse {
    fn from(dto: OrderApprovalDto) -> Self {
        Self {
            id: dto.id,
            saga_id: dto.saga_id,
            order_id: dto.order_id,
            status: dto.status.into(),
            failure_messages: dto.failure_messages,
            created_at: dto.created_at,
        }
    }
}

/// `APPROVED` or `REJECTED` when the restaurant decided; `CANCELLED` when an approved order was
/// cancelled later by the saga compensation (its stock was given back).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[schema(example = "REJECTED")]
pub enum OrderApprovalStatusResponse {
    Approved,
    Rejected,
    Cancelled,
}

impl From<OrderApprovalStatus> for OrderApprovalStatusResponse {
    fn from(status: OrderApprovalStatus) -> Self {
        match status {
            OrderApprovalStatus::Approved => Self::Approved,
            OrderApprovalStatus::Rejected => Self::Rejected,
            OrderApprovalStatus::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct OrderApprovalDataResponse {
    pub data: OrderApprovalResponse,
}

/// Product as known by the restaurant (replicated from the catalog service).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProductResponse {
    #[schema(example = "c2a4f7e8-1b2c-4d5e-8f90-a1b2c3d4e5f6")]
    pub id: Uuid,
    #[schema(example = "Pizza Margherita")]
    pub title: String,
    /// Stock left in the restaurant.
    #[schema(example = 12)]
    pub quantity: i32,
    /// Unit price as a decimal string.
    #[schema(value_type = String, example = "12.50")]
    pub price: Decimal,
}

impl From<ProductDto> for ProductResponse {
    fn from(dto: ProductDto) -> Self {
        Self {
            id: dto.id,
            title: dto.title,
            quantity: dto.quantity,
            price: dto.price,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProductDataResponse {
    pub data: ProductResponse,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProductListDataResponse {
    pub data: Vec<ProductResponse>,
}
