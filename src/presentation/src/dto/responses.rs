use application::dtos::{OrderDto, OrderItemDto};
use domain::enums::OrderStatus;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

const OK_STATUS: &str = "Ok";

/// Envelope used by every successful response: `{ "data": ... }`.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DataResponse<T: ToSchema> {
    pub data: T,
}

#[readonly::make]
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AppInfoResponse {
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

/// Returned when an order is placed. Use the tracking id to follow the order saga.
#[readonly::make]
#[derive(Serialize, Deserialize, ToSchema)]
pub struct OrderResponse {
    #[schema(example = "7c9e6679-7425-40de-944b-e07fc1f90ae7")]
    pub tracking_id: Uuid,
}

impl OrderResponse {
    pub fn new(tracking_id: Uuid) -> Self {
        Self { tracking_id }
    }
}

/// Order lifecycle driven by the saga.
///
/// `PENDING` (waiting for the restaurant) -> `APPROVED` (waiting for the payment) -> `PAID`.
/// A rejection by the restaurant or a failed payment ends in `CANCELLED`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatusResponse {
    Pending,
    Approved,
    Paid,
    Cancelling,
    Cancelled,
}

impl From<OrderStatus> for OrderStatusResponse {
    fn from(status: OrderStatus) -> Self {
        match status {
            OrderStatus::Pending => OrderStatusResponse::Pending,
            OrderStatus::Approved => OrderStatusResponse::Approved,
            OrderStatus::Paid => OrderStatusResponse::Paid,
            OrderStatus::Cancelling => OrderStatusResponse::Cancelling,
            OrderStatus::Canceled => OrderStatusResponse::Cancelled,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct OrderItemResponse {
    #[schema(example = "0411a4a9-1edb-4180-9556-800cb2b84721")]
    pub product_id: Uuid,
    #[schema(example = 2)]
    pub quantity: i32,
    #[schema(example = 30.0)]
    pub price: f64,
    #[schema(example = 60.0)]
    pub sub_total: f64,
}

impl From<OrderItemDto> for OrderItemResponse {
    fn from(item: OrderItemDto) -> Self {
        Self {
            product_id: item.product_id,
            quantity: item.quantity,
            price: item.price.to_f64().unwrap_or_default(),
            sub_total: item.sub_total.to_f64().unwrap_or_default(),
        }
    }
}

/// Current state of an order, as seen through its tracking id.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct OrderTrackingResponse {
    #[schema(example = "eb0163b3-cf88-4053-8c96-986b4f5b9f2e")]
    pub id: Uuid,
    #[schema(example = "7c9e6679-7425-40de-944b-e07fc1f90ae7")]
    pub tracking_id: Uuid,
    #[schema(example = "9b2a3c4d-1e2f-4a5b-8c7d-6e5f4a3b2c1d")]
    pub customer_id: Uuid,
    pub status: OrderStatusResponse,
    #[schema(example = 60.0)]
    pub price: f64,
    pub items: Vec<OrderItemResponse>,
    /// Why the order was cancelled (empty unless `status` is `CANCELLED`).
    #[schema(example = json!([]))]
    pub failure_messages: Vec<String>,
}

impl From<OrderDto> for OrderTrackingResponse {
    fn from(order: OrderDto) -> Self {
        Self {
            id: order.id,
            tracking_id: order.tracking_id,
            customer_id: order.customer_id,
            status: order.status.into(),
            price: order.price.to_f64().unwrap_or_default(),
            items: order
                .items
                .into_iter()
                .map(OrderItemResponse::from)
                .collect(),
            failure_messages: order.failure_messages,
        }
    }
}
