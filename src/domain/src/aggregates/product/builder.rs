use crate::aggregates::category::Category;
use crate::aggregates::product::Product;
use rusty_money::iso::Currency;
use rusty_money::{Money, iso};
use shared::domain::value_objects::ProductId;

pub struct ProductBuilder {
    id: ProductId,
    title: String,
    description: String,
    quantity: i32,
    price: Money<'static, Currency>,
    category: Category,
}

impl Default for ProductBuilder {
    fn default() -> Self {
        Self {
            id: ProductId::new(),
            title: String::new(),
            description: String::new(),
            quantity: 0,
            price: Money::from_minor(0, iso::USD),
            category: Category::default(),
        }
    }
}

impl ProductBuilder {
    pub fn id(mut self, id: ProductId) -> Self {
        self.id = id;
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
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

    pub fn category(mut self, category: Category) -> Self {
        self.category = category;
        self
    }

    pub fn build(self) -> Product {
        Product::new(
            self.id,
            self.title,
            self.description,
            self.quantity,
            self.price,
            self.category,
        )
    }
}
