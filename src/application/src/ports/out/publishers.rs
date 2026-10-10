use domain::events::{ProductCreatedEvent, ProductDeletedEvent, ProductUpdatedEvent};

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait ProductMessagePublisher: Send + Sync {
    fn publish_created(&self, event: ProductCreatedEvent) -> anyhow::Result<()>;
    fn publish_updated(&self, event: ProductUpdatedEvent) -> anyhow::Result<()>;
    fn publish_deleted(&self, event: ProductDeletedEvent) -> anyhow::Result<()>;
}
