use domain::aggregates::order::Order;
use domain::enums::OrderStatus;
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct OrderItemDto {
    pub product_id: Uuid,
    pub quantity: i32,
    pub price: Decimal,
    pub sub_total: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderDto {
    pub id: Uuid,
    pub tracking_id: Uuid,
    pub customer_id: Uuid,
    pub price: Decimal,
    pub status: OrderStatus,
    pub items: Vec<OrderItemDto>,
    pub failure_messages: Vec<String>,
}

impl From<&Order> for OrderDto {
    fn from(order: &Order) -> Self {
        Self {
            id: *order.id().as_uuid(),
            tracking_id: order.tracking_id(),
            customer_id: *order.customer_id().as_uuid(),
            price: *order.price().amount(),
            status: order.status(),
            items: order
                .items()
                .iter()
                .map(|item| OrderItemDto {
                    product_id: *item.product_id().as_uuid(),
                    quantity: item.quantity(),
                    price: *item.price().amount(),
                    sub_total: *item.sub_total().amount(),
                })
                .collect(),
            failure_messages: order.failure_messages().to_vec(),
        }
    }
}
