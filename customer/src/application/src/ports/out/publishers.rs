use domain::events::{CustomerCreatedEvent, CustomerUpdatedEvent};

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait CustomerMessagePublisher: Send + Sync {
    fn publish_created(&self, event: CustomerCreatedEvent) -> anyhow::Result<()>;
    fn publish_updated(&self, event: CustomerUpdatedEvent) -> anyhow::Result<()>;
}
