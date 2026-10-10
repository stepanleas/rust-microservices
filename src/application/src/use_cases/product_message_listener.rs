use crate::dtos::ProductDto;
use crate::ports::r#in::messages::ProductMessageListener;
use crate::ports::out::repositories::ProductRepository;
use domain::aggregates::product::Product;
use shared::domain::value_objects::ProductId;
use shared::error::AppResult;
use std::sync::Arc;
use uuid::Uuid;

/// Keeps the local copy of the catalog products up to date. Saving is an upsert and deleting
/// a missing product is a no-op, so redelivered messages are harmless.
pub struct ApplicationProductMessageListener {
    repository: Arc<dyn ProductRepository>,
}

impl ApplicationProductMessageListener {
    pub fn new(repository: Arc<dyn ProductRepository>) -> Self {
        Self { repository }
    }
}

impl ProductMessageListener for ApplicationProductMessageListener {
    fn product_created(&self, product: ProductDto) -> AppResult<()> {
        let product_id = product.id;
        self.repository.save(&Product::from(product))?;
        tracing::info!("Product is created in restaurant database with id: {product_id}");

        Ok(())
    }

    fn product_updated(&self, product: ProductDto) -> AppResult<()> {
        let product_id = product.id;
        self.repository.save(&Product::from(product))?;
        tracing::info!("Product is updated in restaurant database with id: {product_id}");

        Ok(())
    }

    fn product_deleted(&self, product_id: Uuid) -> AppResult<()> {
        self.repository.delete(ProductId::from_uuid(product_id))?;
        tracing::info!("Product is deleted from restaurant database with id: {product_id}");

        Ok(())
    }
}
