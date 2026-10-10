use super::MessageKind;
use super::ports::{ClaimedBatch, OutboxMessagePublisher, OutboxRepository};
use crate::shutdown::Shutdown;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How often the relay deletes old `COMPLETED` messages.
const CLEANUP_INTERVAL: Duration = Duration::from_secs(300);
/// Rows deleted per statement during cleanup.
const CLEANUP_BATCH: i64 = 5_000;

#[derive(Debug, Clone, Copy)]
pub struct RelaySettings {
    pub poll_interval: Duration,
    pub batch_size: i64,
    pub max_attempts: i32,
    /// `COMPLETED` messages older than this are deleted (`OUTBOX_RETENTION_HOURS`).
    pub retention: Duration,
}

/// Polls the outbox table and relays pending messages to the message broker.
///
/// A message is marked `COMPLETED` only after the broker acknowledged it, so delivery is
/// at-least-once: consumers must be idempotent. Every replica of a service runs a relay: each
/// batch is claimed with `FOR UPDATE SKIP LOCKED`, so two relays never send the same message.
pub struct OutboxRelay<K: MessageKind> {
    repository: Arc<dyn OutboxRepository<K>>,
    publisher: Arc<dyn OutboxMessagePublisher<K>>,
    settings: RelaySettings,
}

impl<K: MessageKind> OutboxRelay<K> {
    pub fn new(
        repository: Arc<dyn OutboxRepository<K>>,
        publisher: Arc<dyn OutboxMessagePublisher<K>>,
        settings: RelaySettings,
    ) -> Self {
        Self {
            repository,
            publisher,
            settings,
        }
    }

    /// Publishes one batch of pending messages and returns how many were published.
    pub fn process_batch(&self) -> anyhow::Result<usize> {
        self.process_batch_until(&Shutdown::new())
    }

    /// Like [`Self::process_batch`], but stops before the next message once `shutdown` is
    /// triggered; the messages not reached yet are released for the next relay.
    fn process_batch_until(&self, shutdown: &Shutdown) -> anyhow::Result<usize> {
        let ClaimedBatch {
            messages,
            mut transaction,
        } = self.repository.claim_batch(self.settings.batch_size)?;
        let mut published = 0;

        for message in messages {
            if shutdown.is_triggered() {
                break;
            }
            let _span = tracing::info_span!(
                "outbox_publish",
                message_id = %message.id(),
                saga_id = %message.saga_id(),
                message_type = message.message_type().as_str(),
            )
            .entered();

            match self.publisher.publish(&message) {
                Ok(()) => {
                    transaction.mark_completed(message.id())?;
                    published += 1;
                    tracing::info!("Outbox message published");
                }
                Err(error) => {
                    let status = transaction
                        .mark_attempt_failed(message.id(), self.settings.max_attempts)?;
                    tracing::error!(
                        "Failed to publish outbox message (attempt {} of {}, now {}): {}",
                        message.attempts() + 1,
                        self.settings.max_attempts,
                        status.as_str(),
                        error,
                    );
                }
            }
        }

        transaction.commit()?;
        Ok(published)
    }

    /// Deletes the `COMPLETED` messages older than the retention; returns how many.
    pub fn delete_expired(&self) -> anyhow::Result<usize> {
        let before = chrono::Utc::now()
            - chrono::Duration::from_std(self.settings.retention).unwrap_or(chrono::Duration::MAX);
        let mut deleted = 0;

        loop {
            let batch = self.repository.delete_completed(before, CLEANUP_BATCH)?;
            deleted += batch;
            if (batch as i64) < CLEANUP_BATCH {
                return Ok(deleted);
            }
        }
    }

    /// Runs the relay on a dedicated thread until `shutdown` is triggered; it then finishes
    /// the message being published, commits and exits.
    pub fn spawn(self, shutdown: &Shutdown) -> std::thread::JoinHandle<()> {
        let shutdown = shutdown.clone();

        std::thread::spawn(move || {
            tracing::info!(
                "Outbox relay started, polling every {:?}",
                self.settings.poll_interval
            );
            let mut last_cleanup: Option<Instant> = None;

            loop {
                if let Err(error) = self.process_batch_until(&shutdown) {
                    tracing::error!("Outbox relay failed: {}", error);
                }

                if last_cleanup.is_none_or(|at| at.elapsed() >= CLEANUP_INTERVAL) {
                    last_cleanup = Some(Instant::now());
                    match self.delete_expired() {
                        Ok(0) => {}
                        Ok(deleted) => tracing::info!("Deleted {deleted} old outbox messages"),
                        Err(error) => tracing::error!("Outbox cleanup failed: {}", error),
                    }
                }

                if !shutdown.sleep(self.settings.poll_interval) {
                    tracing::info!("Outbox relay stopped");
                    return;
                }
            }
        })
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::outbox::ports::{
        MockOutboxBatchTransaction, MockOutboxMessagePublisher, MockOutboxRepository,
    };
    use crate::outbox::{OutboxMessage, OutboxStatus};
    use mockall::predicate::eq;
    use uuid::Uuid;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Kind;

    impl MessageKind for Kind {
        fn as_str(&self) -> &'static str {
            "KIND"
        }

        fn parse(_: &str) -> anyhow::Result<Self> {
            Ok(Kind)
        }
    }

    fn settings() -> RelaySettings {
        RelaySettings {
            poll_interval: Duration::from_millis(1),
            batch_size: 10,
            max_attempts: 3,
            retention: Duration::from_secs(3600),
        }
    }

    fn claiming(
        messages: Vec<OutboxMessage<Kind>>,
        transaction: MockOutboxBatchTransaction,
    ) -> MockOutboxRepository<Kind> {
        let mut repository = MockOutboxRepository::<Kind>::new();
        repository
            .expect_claim_batch()
            .with(eq(10))
            .return_once(move |_| {
                Ok(ClaimedBatch {
                    messages,
                    transaction: Box::new(transaction),
                })
            });
        repository
    }

    #[test]
    fn marks_published_messages_completed() -> anyhow::Result<()> {
        let message = OutboxMessage::new(Uuid::new_v4(), Kind, &"payload")?;
        let id = message.id();

        let mut transaction = MockOutboxBatchTransaction::new();
        transaction
            .expect_mark_completed()
            .with(eq(id))
            .times(1)
            .returning(|_| Ok(()));
        transaction.expect_commit().times(1).returning(|| Ok(()));
        let repository = claiming(vec![message], transaction);
        let mut publisher = MockOutboxMessagePublisher::<Kind>::new();
        publisher.expect_publish().times(1).returning(|_| Ok(()));

        let relay = OutboxRelay::new(Arc::new(repository), Arc::new(publisher), settings());

        assert_eq!(1, relay.process_batch()?);
        Ok(())
    }

    #[test]
    fn counts_failed_attempts() -> anyhow::Result<()> {
        let message = OutboxMessage::new(Uuid::new_v4(), Kind, &"payload")?;
        let id = message.id();

        let mut transaction = MockOutboxBatchTransaction::new();
        transaction
            .expect_mark_attempt_failed()
            .with(eq(id), eq(3))
            .times(1)
            .returning(|_, _| Ok(OutboxStatus::Started));
        transaction.expect_mark_completed().never();
        transaction.expect_commit().times(1).returning(|| Ok(()));
        let repository = claiming(vec![message], transaction);
        let mut publisher = MockOutboxMessagePublisher::<Kind>::new();
        publisher
            .expect_publish()
            .returning(|_| Err(anyhow::anyhow!("broker down")));

        let relay = OutboxRelay::new(Arc::new(repository), Arc::new(publisher), settings());

        assert_eq!(0, relay.process_batch()?);
        Ok(())
    }

    #[test]
    fn stops_before_the_next_message_on_shutdown() -> anyhow::Result<()> {
        let messages = vec![
            OutboxMessage::new(Uuid::new_v4(), Kind, &"first")?,
            OutboxMessage::new(Uuid::new_v4(), Kind, &"second")?,
        ];
        let shutdown = Shutdown::new();

        let mut transaction = MockOutboxBatchTransaction::new();
        transaction
            .expect_mark_completed()
            .times(1)
            .returning(|_| Ok(()));
        transaction.expect_commit().times(1).returning(|| Ok(()));
        let repository = claiming(messages, transaction);
        let mut publisher = MockOutboxMessagePublisher::<Kind>::new();
        let trigger = shutdown.clone();
        publisher.expect_publish().times(1).returning(move |_| {
            trigger.trigger();
            Ok(())
        });

        let relay = OutboxRelay::new(Arc::new(repository), Arc::new(publisher), settings());

        assert_eq!(1, relay.process_batch_until(&shutdown)?);
        Ok(())
    }

    #[test]
    fn deletes_expired_messages_in_batches() -> anyhow::Result<()> {
        let mut repository = MockOutboxRepository::<Kind>::new();
        let mut calls = 0;
        repository
            .expect_delete_completed()
            .times(2)
            .returning(move |_, limit| {
                calls += 1;
                Ok(if calls == 1 { limit as usize } else { 7 })
            });

        let relay = OutboxRelay::new(
            Arc::new(repository),
            Arc::new(MockOutboxMessagePublisher::<Kind>::new()),
            settings(),
        );

        assert_eq!(CLEANUP_BATCH as usize + 7, relay.delete_expired()?);
        Ok(())
    }
}
