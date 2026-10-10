use super::{KafkaConsumer, KafkaProducer, dead_letter_topic};
use crate::shutdown::Shutdown;
use std::sync::Arc;
use std::time::Duration;

/// How long one poll waits for a message before polling again.
const POLL_TIMEOUT: Duration = Duration::from_millis(500);

/// Handles the messages of the topics a service consumes (the driving adapter's entry point).
pub trait MessageHandler: Send + Sync {
    fn topics(&self) -> Vec<&'static str>;

    /// Must be idempotent: a message is redelivered when the service stops before committing.
    fn handle(&self, topic: &str, payload: &[u8]) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    /// Attempts per message before it goes to the dead letter topic (at least 1).
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(10),
        }
    }
}

impl RetryPolicy {
    fn backoff(&self, attempt: u32) -> Duration {
        let factor = 2u32.saturating_pow(attempt.saturating_sub(1));
        self.initial_backoff
            .saturating_mul(factor)
            .min(self.max_backoff)
    }
}

/// Consumes the handler's topics until `shutdown` is triggered. Blocking: run it on its own
/// thread.
///
/// Joins the consumer group `group_id`, so the replicas of a service split the partitions.
/// A message's offset is committed only once it was either handled or parked in `<topic>.dlq`.
/// A failing message is retried with backoff first, so a database hiccup does not lose a saga
/// reply. Messages of one partition are handled in order, so a retried message holds back the
/// ones behind it.
///
/// On shutdown the listener finishes the message in hand (a message still being retried is
/// left for redelivery, not dead-lettered), commits the stored offsets and returns.
pub fn listen(
    hosts: Vec<String>,
    group_id: String,
    handler: Arc<dyn MessageHandler>,
    retry: RetryPolicy,
    shutdown: &Shutdown,
) {
    let topics = handler.topics();
    let Some(consumer) = retry_forever("create Kafka consumer", shutdown, || {
        KafkaConsumer::new(hosts.clone(), group_id.clone(), &topics)
    }) else {
        return;
    };
    let mut dead_letters = KafkaProducer::new(hosts);

    tracing::info!("Kafka listener started for topics {:?}", topics);
    let mut poll_backoff = Duration::from_secs(1);

    while !shutdown.is_triggered() {
        let message = match consumer.poll(POLL_TIMEOUT) {
            Ok(Some(message)) => {
                poll_backoff = Duration::from_secs(1);
                message
            }
            Ok(None) => continue,
            Err(error) => {
                tracing::warn!(
                    "Kafka poll failed: {}. Retrying in {:?}...",
                    error,
                    poll_backoff
                );
                shutdown.sleep(poll_backoff);
                poll_backoff = (poll_backoff * 2).min(Duration::from_secs(30));
                continue;
            }
        };

        let _span = tracing::info_span!(
            "kafka_message",
            topic = %message.topic,
            partition = message.partition,
            offset = message.offset,
            key = %String::from_utf8_lossy(&message.key),
        )
        .entered();

        let outcome = handle_with_retry(
            handler.as_ref(),
            &message.topic,
            &message.payload,
            &retry,
            |backoff| shutdown.sleep(backoff),
        );
        match outcome {
            Outcome::Handled => {}
            Outcome::Interrupted => {
                tracing::info!("Shutting down; the message will be redelivered");
                break;
            }
            Outcome::GaveUp(error) => {
                let dlq = dead_letter_topic(&message.topic);
                tracing::error!("Giving up on message: {}. Moving it to {}", error, dlq);
                let parked = retry_forever("publish to dead letter topic", shutdown, || {
                    dead_letters.send(
                        &dlq,
                        &String::from_utf8_lossy(&message.key),
                        &message.payload,
                    )
                });
                if parked.is_none() {
                    break;
                }
            }
        }

        if let Err(error) = consumer.store_offset(&message) {
            // Happens when the partition was revoked meanwhile; its new owner re-reads it.
            tracing::warn!("Failed to store offset: {}", error);
        }
    }

    match consumer.commit() {
        Ok(()) => tracing::info!("Kafka listener stopped, offsets committed"),
        Err(error) if error.to_string().contains("NoOffset") => {
            tracing::info!("Kafka listener stopped, no new offsets to commit")
        }
        Err(error) => tracing::warn!("Kafka listener stopped, final commit failed: {error}"),
    }
}

#[derive(Debug)]
enum Outcome {
    Handled,
    /// The handler kept failing; the last error.
    GaveUp(anyhow::Error),
    /// Shutdown was triggered while waiting to retry.
    Interrupted,
}

/// Calls the handler until it succeeds or the attempts are used up. `sleep` waits between
/// attempts and returns `false` to stop retrying.
fn handle_with_retry(
    handler: &dyn MessageHandler,
    topic: &str,
    payload: &[u8],
    retry: &RetryPolicy,
    sleep: impl Fn(Duration) -> bool,
) -> Outcome {
    let mut attempt = 1;
    loop {
        match handler.handle(topic, payload) {
            Ok(()) => return Outcome::Handled,
            Err(error) if attempt >= retry.max_attempts.max(1) => return Outcome::GaveUp(error),
            Err(error) => {
                let backoff = retry.backoff(attempt);
                tracing::warn!(
                    "Failed to handle message of {} (attempt {} of {}): {}. Retrying in {:?}",
                    topic,
                    attempt,
                    retry.max_attempts,
                    error,
                    backoff,
                );
                if !sleep(backoff) {
                    return Outcome::Interrupted;
                }
                attempt += 1;
            }
        }
    }
}

/// Retries `operation` with backoff until it succeeds; `None` if shutdown came first.
fn retry_forever<T>(
    what: &str,
    shutdown: &Shutdown,
    mut operation: impl FnMut() -> anyhow::Result<T>,
) -> Option<T> {
    let mut backoff = Duration::from_secs(1);
    loop {
        match operation() {
            Ok(value) => return Some(value),
            Err(error) => {
                tracing::warn!(
                    "Failed to {}: {}. Retrying in {:?}...",
                    what,
                    error,
                    backoff
                );
                if !shutdown.sleep(backoff) {
                    return None;
                }
                backoff = (backoff * 2).min(Duration::from_secs(30));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct FailingHandler {
        failures_before_success: u32,
        calls: AtomicU32,
    }

    impl MessageHandler for FailingHandler {
        fn topics(&self) -> Vec<&'static str> {
            vec!["topic"]
        }

        fn handle(&self, _: &str, _: &[u8]) -> anyhow::Result<()> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            if call <= self.failures_before_success {
                anyhow::bail!("failure {call}");
            }
            Ok(())
        }
    }

    fn policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_millis(150),
        }
    }

    #[test]
    fn retries_until_the_handler_succeeds() {
        let handler = FailingHandler {
            failures_before_success: 2,
            calls: AtomicU32::new(0),
        };
        let sleeps = Mutex::new(vec![]);

        let result = handle_with_retry(&handler, "topic", b"", &policy(), |d| {
            sleeps.lock().unwrap().push(d);
            true
        });

        assert!(matches!(result, Outcome::Handled));
        assert_eq!(3, handler.calls.load(Ordering::SeqCst));
        assert_eq!(
            vec![Duration::from_millis(100), Duration::from_millis(150)],
            *sleeps.lock().unwrap()
        );
    }

    #[test]
    fn gives_up_after_max_attempts() {
        let handler = FailingHandler {
            failures_before_success: 10,
            calls: AtomicU32::new(0),
        };

        let result = handle_with_retry(&handler, "topic", b"", &policy(), |_| true);

        let Outcome::GaveUp(error) = result else {
            panic!("expected GaveUp, got {result:?}");
        };
        assert_eq!("failure 3", error.to_string());
        assert_eq!(3, handler.calls.load(Ordering::SeqCst));
    }

    #[test]
    fn stops_retrying_on_shutdown() {
        let handler = FailingHandler {
            failures_before_success: 10,
            calls: AtomicU32::new(0),
        };

        let result = handle_with_retry(&handler, "topic", b"", &policy(), |_| false);

        assert!(matches!(result, Outcome::Interrupted));
        assert_eq!(1, handler.calls.load(Ordering::SeqCst));
    }
}
