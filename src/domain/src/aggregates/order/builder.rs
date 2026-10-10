use crate::aggregates::order::Order;
use crate::aggregates::order::OrderItem;
use crate::enums::OrderStatus;
use rusty_money::iso::Currency;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CustomerId, OrderId, OrderItemId, ProductId};
use uuid::Uuid;

pub struct OrderBuilder {
    id: OrderId,
    customer_id: CustomerId,
    tracking_id: Uuid,
    price: Money<'static, Currency>,
    items: Vec<OrderItem>,
    status: OrderStatus,
    failure_messages: Vec<String>,
}

impl OrderBuilder {
    pub fn id(mut self, id: OrderId) -> Self {
        self.id = id;
        self
    }

    pub fn customer_id(mut self, id: CustomerId) -> Self {
        self.customer_id = id;
        self
    }

    pub fn tracking_id(mut self, tracking_id: Uuid) -> Self {
        self.tracking_id = tracking_id;
        self
    }

    pub fn price(mut self, price: Money<'static, Currency>) -> Self {
        self.price = price;
        self
    }

    pub fn items(mut self, items: Vec<OrderItem>) -> Self {
        self.items = items;
        self
    }

    pub fn status(mut self, status: OrderStatus) -> Self {
        self.status = status;
        self
    }

    pub fn failure_messages(mut self, failure_messages: Vec<String>) -> Self {
        self.failure_messages = failure_messages;
        self
    }

    pub fn build(self) -> Order {
        Order::restore(
            self.id,
            self.customer_id,
            self.tracking_id,
            self.price,
            self.items,
            self.status,
            self.failure_messages,
        )
    }
}

impl Default for OrderBuilder {
    fn default() -> Self {
        Self {
            id: OrderId::new(),
            customer_id: CustomerId::new(),
            tracking_id: Uuid::new_v4(),
            price: Money::from_minor(0, iso::USD),
            items: vec![],
            status: OrderStatus::Pending,
            failure_messages: vec![],
        }
    }
}

pub struct OrderItemBuilder {
    id: OrderItemId,
    order_id: OrderId,
    product_id: ProductId,
    quantity: i32,
    price: Money<'static, Currency>,
    sub_total: Money<'static, Currency>,
}

impl Default for OrderItemBuilder {
    fn default() -> Self {
        Self {
            id: OrderItemId::new(),
            order_id: OrderId::new(),
            product_id: ProductId::new(),
            quantity: 0,
            price: Money::from_minor(0, iso::USD),
            sub_total: Money::from_minor(0, iso::USD),
        }
    }
}

impl OrderItemBuilder {
    pub fn id(mut self, id: OrderItemId) -> Self {
        self.id = id;
        self
    }

    pub fn order_id(mut self, id: OrderId) -> Self {
        self.order_id = id;
        self
    }

    pub fn product_id(mut self, id: ProductId) -> Self {
        self.product_id = id;
        self
    }

    pub fn quantity(mut self, quantity: i32) -> Self {
        self.quantity = quantity;
        self
    }

    pub fn price(mut self, price: Money<'static, Currency>) -> Self {
        self.price = price;
        self
    }

    pub fn sub_total(mut self, sub_total: Money<'static, Currency>) -> Self {
        self.sub_total = sub_total;
        self
    }

    pub fn build(self) -> OrderItem {
        OrderItem::new(
            self.id,
            self.order_id,
            self.product_id,
            self.quantity,
            self.price,
            self.sub_total,
        )
    }
}
