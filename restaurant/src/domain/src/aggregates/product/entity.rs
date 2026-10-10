use crate::aggregates::product::ProductBuilder;
use crate::error::DomainError;
use rust_decimal::Decimal;
use shared::domain::value_objects::ProductId;

#[derive(Debug, Clone, PartialEq)]
pub struct Product {
    id: ProductId,
    title: String,
    quantity: i32,
    price: Decimal,
}

impl Product {
    pub fn builder() -> ProductBuilder {
        ProductBuilder::default()
    }

    pub fn new(id: ProductId, title: String, quantity: i32, price: Decimal) -> Self {
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

    pub fn price(&self) -> Decimal {
        self.price
    }

    pub fn has_stock_for(&self, quantity: i32) -> bool {
        quantity <= self.quantity
    }

    pub fn decrease_stock(&mut self, quantity: i32) -> Result<(), DomainError> {
        if quantity <= 0 {
            return Err(DomainError::RestaurantDomainError {
                message: format!("Quantity must be greater than zero, got {quantity}"),
            });
        }

        if !self.has_stock_for(quantity) {
            return Err(DomainError::RestaurantDomainError {
                message: format!(
                    "Insufficient stock for product {}: requested {}, available {}",
                    self.title, quantity, self.quantity,
                ),
            });
        }

        self.quantity -= quantity;
        Ok(())
    }

    /// Gives stock back, e.g. when an approved order is cancelled.
    pub fn increase_stock(&mut self, quantity: i32) -> Result<(), DomainError> {
        if quantity <= 0 {
            return Err(DomainError::RestaurantDomainError {
                message: format!("Quantity must be greater than zero, got {quantity}"),
            });
        }

        self.quantity = self.quantity.checked_add(quantity).ok_or_else(|| {
            DomainError::RestaurantDomainError {
                message: format!(
                    "Stock of product {} would overflow: {} + {}",
                    self.title, self.quantity, quantity,
                ),
            }
        })?;
        Ok(())
    }
}
