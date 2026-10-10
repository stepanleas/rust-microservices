use super::load_category;
use crate::commands::UpdateProductCommand;
use crate::dtos::ProductDto;
use crate::ports::out::publishers::ProductMessagePublisher;
use crate::ports::out::repositories::{CategoryRepository, ProductRepository};
use domain::aggregates::product::Product;
use domain::events::ProductUpdatedEvent;
use rusty_money::{Money, iso};
use shared::domain::value_objects::ProductId;
use shared::error::AppResult;
use std::sync::Arc;

pub struct UpdateProduct {
    product_repository: Arc<dyn ProductRepository>,
    category_repository: Arc<dyn CategoryRepository>,
    message_publisher: Arc<dyn ProductMessagePublisher>,
}

impl UpdateProduct {
    pub fn new(
        product_repository: Arc<dyn ProductRepository>,
        category_repository: Arc<dyn CategoryRepository>,
        message_publisher: Arc<dyn ProductMessagePublisher>,
    ) -> Self {
        Self {
            product_repository,
            category_repository,
            message_publisher,
        }
    }

    /// Upserts the product, then publishes `product-updated`.
    ///
    /// If publishing fails the product stays saved and the error is returned (HTTP 500).
    // TODO: Add transactional outbox
    pub fn execute(&self, command: UpdateProductCommand) -> AppResult<ProductDto> {
        let category = load_category(self.category_repository.as_ref(), command.category_id())?;

        let product = Product::builder()
            .id(ProductId::from_uuid(command.id()))
            .title(command.title())
            .description(command.description())
            .quantity(command.quantity())
            .price(Money::from_decimal(command.price(), iso::USD))
            .category(category)
            .build();

        let saved_product = self.product_repository.save(product)?;
        tracing::info!("Product with id: {} updated", saved_product.id());

        let product_dto = ProductDto::from(&saved_product);
        self.message_publisher
            .publish_updated(ProductUpdatedEvent::new(saved_product))?;

        Ok(product_dto)
    }
}
