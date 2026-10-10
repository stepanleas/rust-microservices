use crate::aggregates::product::Product;
use rust_decimal::Decimal;
use shared::domain::value_objects::ProductId;

#[derive(Default)]
pub struct ProductBuilder {
    id: ProductId,
    title: String,
    quantity: i32,
    price: Decimal,
}

impl ProductBuilder {
    pub fn id(mut self, id: ProductId) -> Self {
        self.id = id;
        self
    }

    pub fn title(mut self, title: String) -> Self {
        self.title = title;
        self
    }

    pub fn quantity(mut self, quantity: i32) -> Self {
        self.quantity = quantity;
        self
    }

    pub fn price(mut self, price: Decimal) -> Self {
        self.price = price;
        self
    }

    pub fn build(self) -> Product {
        Product::new(self.id, self.title, self.quantity, self.price)
    }
}
