use crate::aggregates::category::Category;
use crate::aggregates::product::ProductBuilder;
use rusty_money::Money;
use rusty_money::iso::Currency;
use shared::domain::value_objects::ProductId;

#[derive(Debug, PartialEq, Clone)]
pub struct Product {
    id: ProductId,
    title: String,
    description: String,
    quantity: i32,
    price: Money<'static, Currency>,
    category: Category,
}

impl Product {
    pub fn builder() -> ProductBuilder {
        ProductBuilder::default()
    }

    pub fn new(
        id: ProductId,
        title: String,
        description: String,
        quantity: i32,
        price: Money<'static, Currency>,
        category: Category,
    ) -> Self {
        Product {
            id,
            title,
            description,
            quantity,
            price,
            category,
        }
    }

    pub fn id(&self) -> ProductId {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn quantity(&self) -> i32 {
        self.quantity
    }

    pub fn price(&self) -> &Money<'static, Currency> {
        &self.price
    }

    pub fn category(&self) -> &Category {
        &self.category
    }
}
