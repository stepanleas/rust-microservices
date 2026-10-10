use application::commands::{CreateOrderCommand, CreateOrderItemDto};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::{Deserialize, Serialize};
use shared::web::ApiError;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Places an order. `price` must equal the sum of the items' `sub_total`,
/// and each `sub_total` must equal `price * quantity`.
#[readonly::make]
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(example = json!({
    "customer_id": "9b2a3c4d-1e2f-4a5b-8c7d-6e5f4a3b2c1d",
    "price": 60.0,
    "items": [
        { "product_id": "0411a4a9-1edb-4180-9556-800cb2b84721", "quantity": 2, "price": 30.0, "sub_total": 60.0 }
    ]
}))]
pub struct CreateOrderRequest {
    pub customer_id: Uuid,

    #[validate(range(min = 1.0, message = "Price must be greater than 0"))]
    pub price: f64,

    #[validate(
        length(min = 1, message = "Order must contain at least one item"),
        nested
    )]
    pub items: Vec<CreateOrderItem>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateOrderItem {
    pub product_id: Uuid,

    #[validate(range(min = 1, message = "Quantity must be greater than 0"))]
    pub quantity: i32,

    #[validate(range(min = 1.0, message = "Price must be greater than 0"))]
    pub price: f64,

    #[validate(range(min = 1.0, message = "Price must be greater than 0"))]
    pub sub_total: f64,
}

fn to_decimal(field: &str, value: f64) -> Result<Decimal, ApiError> {
    Decimal::from_f64(value)
        .ok_or_else(|| ApiError::bad_request(format!("Invalid {field} value: {value}")))
}

impl TryFrom<CreateOrderRequest> for CreateOrderCommand {
    type Error = ApiError;

    fn try_from(request: CreateOrderRequest) -> Result<Self, Self::Error> {
        let items = request
            .items
            .iter()
            .map(CreateOrderItemDto::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(CreateOrderCommand::new(
            request.customer_id,
            to_decimal("price", request.price)?,
            items,
        ))
    }
}

impl TryFrom<&CreateOrderItem> for CreateOrderItemDto {
    type Error = ApiError;

    fn try_from(item: &CreateOrderItem) -> Result<Self, Self::Error> {
        Ok(CreateOrderItemDto::new(
            item.product_id,
            item.quantity,
            to_decimal("product price", item.price)?,
            to_decimal("subtotal price", item.sub_total)?,
        ))
    }
}
