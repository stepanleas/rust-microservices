use super::{MessageKind, OutboxMessage, OutboxStatus};
use uuid::Uuid;

#[cfg_attr(feature = "mocks", mockall::automock)]
pub trait OutboxRepository<K: MessageKind>: Send + Sync {
    /// Oldest messages first.
    fn find_by_status(
        &self,
        status: OutboxStatus,
        limit: i64,
    ) -> anyhow::Result<Vec<OutboxMessage<K>>>;

    fn mark_completed(&self, id: Uuid) -> anyhow::Result<()>;

    /// Increments the attempt counter and marks the message `FAILED` once `max_attempts` is
    /// reached. Returns the resulting status.
    fn mark_attempt_failed(&self, id: Uuid, max_attempts: i32) -> anyhow::Result<OutboxStatus>;
}

#[cfg_attr(feature = "mocks", mockall::automock)]
pub trait OutboxMessagePublisher<K: MessageKind>: Send + Sync {
    fn publish(&self, message: &OutboxMessage<K>) -> anyhow::Result<()>;
}
