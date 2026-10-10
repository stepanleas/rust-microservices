use crate::ports::r#in::messages::ProductMessageListener;
use crate::ports::out::repositories::ProductRepository;
use domain::aggregates::product::Product;
use shared::domain::value_objects::ProductId;
use shared::error::AppResult;
use std::sync::Arc;

/// Keeps the local copy of the products (owned by the catalog service) up to date.
/// Saving is an upsert and deleting a missing product is a no-op, so redelivered events are
/// harmless.
pub struct SyncProduct {
    repository: Arc<dyn ProductRepository>,
}

impl SyncProduct {
    pub fn new(repository: Arc<dyn ProductRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, product: Product) -> AppResult<()> {
        let product_id = product.id();
        self.repository.save(product)?;
        tracing::info!("Product {} saved in order database", product_id);

        Ok(())
    }

    pub fn delete(&self, product_id: ProductId) -> AppResult<()> {
        self.repository.delete(product_id)?;
        tracing::info!("Product {} deleted from order database", product_id);

        Ok(())
    }
}

impl ProductMessageListener for SyncProduct {
    fn product_created(&self, product: Product) -> AppResult<()> {
        self.execute(product)
    }

    fn product_updated(&self, product: Product) -> AppResult<()> {
        self.execute(product)
    }

    fn product_deleted(&self, product_id: ProductId) -> AppResult<()> {
        self.delete(product_id)
    }
}
