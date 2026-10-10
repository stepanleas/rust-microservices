use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[readonly::make]
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({
    "order_id": "0411a4a9-1edb-4180-9556-800cb2b84721",
    "customer_id": "eb0163b3-cf88-4053-8c96-986b4f5b9f2e",
    "price": 150.0
}))]
pub struct CreatePaymentRequest {
    /// Order to pay for. Only one payment per order is allowed.
    pub order_id: Uuid,
    /// Customer that pays.
    pub customer_id: Uuid,
    /// Amount to charge, must be greater than zero.
    #[validate(range(min = 0.01, message = "Price must be greater than zero"))]
    pub price: f64,
}

#[readonly::make]
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({
    "order_id": "0411a4a9-1edb-4180-9556-800cb2b84721",
    "customer_id": "eb0163b3-cf88-4053-8c96-986b4f5b9f2e",
    "price": 150.0,
    "payment_status": "CANCELLED"
}))]
pub struct UpdatePaymentRequest {
    pub order_id: Uuid,
    pub customer_id: Uuid,
    #[validate(range(min = 0.01, message = "Price must be greater than zero"))]
    pub price: f64,
    /// One of `COMPLETED`, `FAILED`, `CANCELLED` (case insensitive).
    #[validate(length(
        min = 1,
        max = 50,
        message = "Payment status must be between 1 and 50 characters"
    ))]
    pub payment_status: String,
}
