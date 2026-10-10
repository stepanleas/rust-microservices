use super::ports::{OutboxMessagePublisher, OutboxRepository};
use super::{MessageKind, OutboxStatus};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct RelaySettings {
    pub poll_interval: Duration,
    pub batch_size: i64,
    pub max_attempts: i32,
}

/// Polls the outbox table and relays pending messages to the message broker.
///
/// A message is marked `COMPLETED` only after the broker acknowledged it, so delivery is
/// at-least-once: consumers must be idempotent. Assumes a single relay per service; a second
/// one would only cause duplicate sends.
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
        let messages = self
            .repository
            .find_by_status(OutboxStatus::Started, self.settings.batch_size)?;
        let mut published = 0;

        for message in messages {
            match self.publisher.publish(&message) {
                Ok(()) => {
                    self.repository.mark_completed(message.id())?;
                    published += 1;
                    tracing::info!(
                        "Outbox message {} ({}) published for saga {}",
                        message.id(),
                        message.message_type().as_str(),
                        message.saga_id(),
                    );
                }
                Err(error) => {
                    let status = self
                        .repository
                        .mark_attempt_failed(message.id(), self.settings.max_attempts)?;
                    tracing::error!(
                        "Failed to publish outbox message {} (attempt {} of {}, now {}): {}",
                        message.id(),
                        message.attempts() + 1,
                        self.settings.max_attempts,
                        status.as_str(),
                        error,
                    );
                }
            }
        }

        Ok(published)
    }

    /// Runs the relay on a dedicated thread, forever.
    pub fn spawn(self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            tracing::info!(
                "Outbox relay started, polling every {:?}",
                self.settings.poll_interval
            );
            loop {
                if let Err(error) = self.process_batch() {
                    tracing::error!("Outbox relay failed: {}", error);
                }
                std::thread::sleep(self.settings.poll_interval);
            }
        })
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::outbox::OutboxMessage;
    use crate::outbox::ports::{MockOutboxMessagePublisher, MockOutboxRepository};
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
        }
    }

    #[test]
    fn marks_published_messages_completed() -> anyhow::Result<()> {
        let message = OutboxMessage::new(Uuid::new_v4(), Kind, &"payload")?;
        let id = message.id();

        let mut repository = MockOutboxRepository::<Kind>::new();
        repository
            .expect_find_by_status()
            .with(eq(OutboxStatus::Started), eq(10))
            .return_once(move |_, _| Ok(vec![message]));
        repository
            .expect_mark_completed()
            .with(eq(id))
            .times(1)
            .returning(|_| Ok(()));
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

        let mut repository = MockOutboxRepository::<Kind>::new();
        repository
            .expect_find_by_status()
            .return_once(move |_, _| Ok(vec![message]));
        repository
            .expect_mark_attempt_failed()
            .with(eq(id), eq(3))
            .times(1)
            .returning(|_, _| Ok(OutboxStatus::Started));
        repository.expect_mark_completed().never();
        let mut publisher = MockOutboxMessagePublisher::<Kind>::new();
        publisher
            .expect_publish()
            .returning(|_| Err(anyhow::anyhow!("broker down")));

        let relay = OutboxRelay::new(Arc::new(repository), Arc::new(publisher), settings());

        assert_eq!(0, relay.process_batch()?);
        Ok(())
    }
}
