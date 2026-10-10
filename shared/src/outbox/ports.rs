use super::{MessageKind, OutboxMessage, OutboxStatus};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[cfg_attr(feature = "mocks", mockall::automock)]
pub trait OutboxRepository<K: MessageKind>: Send + Sync {
    /// Oldest messages first.
    fn find_by_status(
        &self,
        status: OutboxStatus,
        limit: i64,
    ) -> anyhow::Result<Vec<OutboxMessage<K>>>;

    /// Locks up to `limit` `STARTED` messages (oldest first) that no other relay holds, so
    /// several replicas of a service can relay the same outbox without sending a message twice.
    /// The lock lasts until the returned batch is committed or dropped.
    fn claim_batch(&self, limit: i64) -> anyhow::Result<ClaimedBatch<K>>;

    fn mark_completed(&self, id: Uuid) -> anyhow::Result<()>;

    /// Increments the attempt counter and marks the message `FAILED` once `max_attempts` is
    /// reached. Returns the resulting status.
    fn mark_attempt_failed(&self, id: Uuid, max_attempts: i32) -> anyhow::Result<OutboxStatus>;

    /// Deletes up to `limit` `COMPLETED` messages processed before `before`; returns how many.
    fn delete_completed(&self, before: DateTime<Utc>, limit: i64) -> anyhow::Result<usize>;
}

/// Messages claimed by one relay, with the transaction that holds their locks.
pub struct ClaimedBatch<K: MessageKind> {
    pub messages: Vec<OutboxMessage<K>>,
    pub transaction: Box<dyn OutboxBatchTransaction>,
}

/// Records the outcome of each claimed message. Nothing is saved until [`Self::commit`];
/// dropping it rolls back and releases the locks, so the messages are relayed again.
#[cfg_attr(feature = "mocks", mockall::automock)]
pub trait OutboxBatchTransaction: Send {
    fn mark_completed(&mut self, id: Uuid) -> anyhow::Result<()>;

    /// See [`OutboxRepository::mark_attempt_failed`].
    fn mark_attempt_failed(&mut self, id: Uuid, max_attempts: i32) -> anyhow::Result<OutboxStatus>;

    fn commit(self: Box<Self>) -> anyhow::Result<()>;
}

#[cfg_attr(feature = "mocks", mockall::automock)]
pub trait OutboxMessagePublisher<K: MessageKind>: Send + Sync {
    fn publish(&self, message: &OutboxMessage<K>) -> anyhow::Result<()>;
}
