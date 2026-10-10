use crate::aggregates::order::OrderItemBuilder;
use rusty_money::Money;
use rusty_money::iso::Currency;
use shared::domain::value_objects::{OrderId, OrderItemId, ProductId};

pub struct OrderItem {
    id: OrderItemId,
    order_id: OrderId,
    product_id: ProductId,
    quantity: i32,
    price: Money<'static, Currency>,
    sub_total: Money<'static, Currency>,
}

impl OrderItem {
    pub fn builder() -> OrderItemBuilder {
        OrderItemBuilder::default()
    }

    pub fn new(
        id: OrderItemId,
        order_id: OrderId,
        product_id: ProductId,
        quantity: i32,
        price: Money<'static, Currency>,
        sub_total: Money<'static, Currency>,
    ) -> Self {
        Self {
            id,
            order_id,
            product_id,
            quantity,
            price,
            sub_total,
        }
    }

    pub fn id(&self) -> OrderItemId {
        self.id
    }

    pub fn order_id(&self) -> OrderId {
        self.order_id
    }

    pub fn product_id(&self) -> ProductId {
        self.product_id
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }

    pub fn price(&self) -> &Money<'static, Currency> {
        &self.price
    }

    pub fn sub_total(&self) -> &Money<'static, Currency> {
        &self.sub_total
    }

    pub fn is_price_valid(&self) -> bool {
        if self.price.is_zero() {
            return false;
        }

        let multiplied = match self.price.mul(self.quantity) {
            Ok(value) => value,
            Err(_) => return false,
        };

        multiplied == self.sub_total
    }
}
