use domain::events::{PaymentCreatedEvent, PaymentUpdatedEvent};

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait PaymentMessagePublisher: Send + Sync {
    fn publish_created(&self, event: PaymentCreatedEvent) -> anyhow::Result<()>;
    fn publish_updated(&self, event: PaymentUpdatedEvent) -> anyhow::Result<()>;
}
