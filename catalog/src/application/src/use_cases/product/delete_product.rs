use crate::commands::DeleteProductCommand;
use crate::ports::out::publishers::ProductMessagePublisher;
use crate::ports::out::repositories::ProductRepository;
use domain::events::ProductDeletedEvent;
use shared::domain::value_objects::ProductId;
use shared::error::AppResult;
use std::sync::Arc;

pub struct DeleteProduct {
    repository: Arc<dyn ProductRepository>,
    message_publisher: Arc<dyn ProductMessagePublisher>,
}

impl DeleteProduct {
    pub fn new(
        repository: Arc<dyn ProductRepository>,
        message_publisher: Arc<dyn ProductMessagePublisher>,
    ) -> Self {
        Self {
            repository,
            message_publisher,
        }
    }

    /// Deletes the product, then publishes `product-deleted`.
    ///
    /// If publishing fails the product stays deleted and the error is returned (HTTP 500).
    // TODO: Add transactional outbox
    pub fn execute(&self, command: DeleteProductCommand) -> AppResult<()> {
        let product_id = ProductId::from_uuid(command.id());

        self.repository.delete(product_id)?;
        tracing::info!("Product with id: {} deleted", product_id);

        self.message_publisher
            .publish_deleted(ProductDeletedEvent::new(product_id))?;

        Ok(())
    }
}
