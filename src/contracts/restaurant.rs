use super::avro_message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestaurantApprovalProductAvroModel {
    pub id: String,
    pub quantity: i32,
}

/// order -> restaurant: reserve the products of an order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestaurantApprovalRequestAvroModel {
    pub id: String,
    pub saga_id: String,
    pub order_id: String,
    pub customer_id: String,
    pub price: String,
    pub products: Vec<RestaurantApprovalProductAvroModel>,
    pub created_at: String,
}

/// restaurant -> order: `order_approval_status` is `APPROVED` or `REJECTED`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestaurantApprovalResponseAvroModel {
    pub id: String,
    pub saga_id: String,
    pub order_id: String,
    pub order_approval_status: String,
    pub failure_messages: Vec<String>,
    pub created_at: String,
}

/// order -> restaurant (compensation): an approved order was cancelled, give the stock back.
///
/// Carries the products so the restaurant does not have to have stored them. The restaurant
/// must apply it at most once per order (it is delivered at least once).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestaurantApprovalCancelRequestAvroModel {
    pub id: String,
    pub saga_id: String,
    pub order_id: String,
    pub products: Vec<RestaurantApprovalProductAvroModel>,
    pub created_at: String,
}

avro_message!(
    RestaurantApprovalRequestAvroModel,
    "restaurant_approval_request_avro_model.avsc"
);
avro_message!(
    RestaurantApprovalResponseAvroModel,
    "restaurant_approval_response_avro_model.avsc"
);
avro_message!(
    RestaurantApprovalCancelRequestAvroModel,
    "restaurant_approval_cancel_request_avro_model.avsc"
);
