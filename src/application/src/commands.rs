use rust_decimal::Decimal;
use uuid::Uuid;

/// Asks the restaurant to approve an order. Built from a `restaurant-approval-request` message.
#[derive(Debug, Clone, PartialEq)]
pub struct ApproveOrderCommand {
    saga_id: Uuid,
    order_id: Uuid,
    customer_id: Uuid,
    price: Decimal,
    products: Vec<OrderProductDto>,
}

impl ApproveOrderCommand {
    pub fn new(
        saga_id: Uuid,
        order_id: Uuid,
        customer_id: Uuid,
        price: Decimal,
        products: Vec<OrderProductDto>,
    ) -> Self {
        Self {
            saga_id,
            order_id,
            customer_id,
            price,
            products,
        }
    }

    pub fn saga_id(&self) -> Uuid {
        self.saga_id
    }

    pub fn order_id(&self) -> Uuid {
        self.order_id
    }

    pub fn customer_id(&self) -> Uuid {
        self.customer_id
    }

    pub fn price(&self) -> Decimal {
        self.price
    }

    pub fn products(&self) -> &[OrderProductDto] {
        &self.products
    }
}

/// Compensation: an approved order was cancelled, give its stock back.
/// Built from a `restaurant-approval-cancel-request` message.
#[derive(Debug, Clone, PartialEq)]
pub struct CancelOrderApprovalCommand {
    saga_id: Uuid,
    order_id: Uuid,
    products: Vec<OrderProductDto>,
}

impl CancelOrderApprovalCommand {
    pub fn new(saga_id: Uuid, order_id: Uuid, products: Vec<OrderProductDto>) -> Self {
        Self {
            saga_id,
            order_id,
            products,
        }
    }

    pub fn saga_id(&self) -> Uuid {
        self.saga_id
    }

    pub fn order_id(&self) -> Uuid {
        self.order_id
    }

    pub fn products(&self) -> &[OrderProductDto] {
        &self.products
    }
}

/// A product line of an order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderProductDto {
    product_id: Uuid,
    quantity: i32,
}

impl OrderProductDto {
    pub fn new(product_id: Uuid, quantity: i32) -> Self {
        Self {
            product_id,
            quantity,
        }
    }

    pub fn product_id(&self) -> Uuid {
        self.product_id
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }
}
