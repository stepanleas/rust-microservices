use crate::aggregates::product::ProductBuilder;
use rusty_money::Money;
use rusty_money::iso::Currency;
use shared::domain::value_objects::ProductId;

pub struct Product {
    id: ProductId,
    title: String,
    quantity: i32,
    price: Money<'static, Currency>,
}

impl Product {
    pub fn builder() -> ProductBuilder {
        ProductBuilder::default()
    }

    pub fn new(
        id: ProductId,
        title: String,
        quantity: i32,
        price: Money<'static, Currency>,
    ) -> Self {
        Self {
            id,
            title,
            quantity,
            price,
        }
    }

    pub fn id(&self) -> ProductId {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }

    pub fn price(&self) -> &Money<'static, Currency> {
        &self.price
    }
}
