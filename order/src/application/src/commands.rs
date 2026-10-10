use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Clone)]
pub struct CreateOrderCommand {
    customer_id: Uuid,
    price: Decimal,
    items: Vec<CreateOrderItemDto>,
}

impl CreateOrderCommand {
    pub fn new(customer_id: Uuid, price: Decimal, items: Vec<CreateOrderItemDto>) -> Self {
        Self {
            customer_id,
            price,
            items,
        }
    }

    pub fn customer_id(&self) -> Uuid {
        self.customer_id
    }

    pub fn price(&self) -> Decimal {
        self.price
    }

    pub fn items(&self) -> &Vec<CreateOrderItemDto> {
        &self.items
    }
}

#[derive(Debug, Clone)]
pub struct CreateOrderItemDto {
    product_id: Uuid,
    quantity: i32,
    price: Decimal,
    sub_total: Decimal,
}

impl CreateOrderItemDto {
    pub fn new(product_id: Uuid, quantity: i32, price: Decimal, sub_total: Decimal) -> Self {
        Self {
            product_id,
            quantity,
            price,
            sub_total,
        }
    }

    pub fn product_id(&self) -> Uuid {
        self.product_id
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }

    pub fn price(&self) -> Decimal {
        self.price
    }

    pub fn sub_total(&self) -> Decimal {
        self.sub_total
    }
}
