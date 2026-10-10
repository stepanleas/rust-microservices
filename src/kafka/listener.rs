use super::{KafkaConsumer, KafkaProducer, dead_letter_topic};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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

/// Consumes the handler's topics forever. Blocking: run it on its own thread.
///
/// Offsets are committed only once every message of a poll was either handled or parked in
/// `<topic>.dlq`. A failing message is retried with backoff first, so a database hiccup does
/// not lose a saga reply. Messages of one partition are handled in order, so a retried
/// message holds back the ones behind it.
pub fn listen(
    hosts: Vec<String>,
    group_id: String,
    handler: Arc<dyn MessageHandler>,
    retry: RetryPolicy,
) {
    let topics = handler.topics();
    let mut consumer = retry_forever("create Kafka consumer", || {
        KafkaConsumer::new(hosts.clone(), group_id.clone(), &topics)
    });
    let dead_letter_topics: Vec<String> = topics.iter().map(|t| dead_letter_topic(t)).collect();
    let dead_letter_topics: Vec<&str> = dead_letter_topics.iter().map(String::as_str).collect();
    let mut dead_letters = KafkaProducer::new(hosts, &dead_letter_topics);

    tracing::info!("Kafka listener started for topics {:?}", topics);
    let mut poll_backoff = Duration::from_secs(1);

    loop {
        let message_sets = match consumer.poll() {
            Ok(message_sets) => {
                poll_backoff = Duration::from_secs(1);
                message_sets
            }
            Err(error) => {
                tracing::warn!(
                    "Kafka poll failed: {}. Retrying in {:?}...",
                    error,
                    poll_backoff
                );
                thread::sleep(poll_backoff);
                poll_backoff = (poll_backoff * 2).min(Duration::from_secs(30));
                continue;
            }
        };

        for message_set in message_sets.iter() {
            let topic = message_set.topic();
            for message in message_set.messages() {
                let delivered = handle_with_retry(
                    handler.as_ref(),
                    topic,
                    message.value,
                    &retry,
                    thread::sleep,
                );
                if let Err(error) = delivered {
                    let dlq = dead_letter_topic(topic);
                    tracing::error!(
                        "Giving up on message at offset {} of {}: {}. Moving it to {}",
                        message.offset,
                        topic,
                        error,
                        dlq,
                    );
                    retry_forever("publish to dead letter topic", || {
                        dead_letters.send(
                            &dlq,
                            &String::from_utf8_lossy(message.key),
                            message.value,
                        )
                    });
                }
            }

            if let Err(error) = consumer.consume_messageset(message_set) {
                tracing::error!(
                    "Failed to mark message set of {} as consumed: {}",
                    topic,
                    error
                );
            }
        }

        if let Err(error) = consumer.commit_consumed() {
            tracing::error!("Failed to commit consumed messages: {}", error);
        }
    }
}

/// Calls the handler until it succeeds or the attempts are used up; returns the last error.
fn handle_with_retry(
    handler: &dyn MessageHandler,
    topic: &str,
    payload: &[u8],
    retry: &RetryPolicy,
    sleep: impl Fn(Duration),
) -> anyhow::Result<()> {
    let mut attempt = 1;
    loop {
        match handler.handle(topic, payload) {
            Ok(()) => return Ok(()),
            Err(error) if attempt >= retry.max_attempts.max(1) => return Err(error),
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
                sleep(backoff);
                attempt += 1;
            }
        }
    }
}

fn retry_forever<T>(what: &str, mut operation: impl FnMut() -> anyhow::Result<T>) -> T {
    let mut backoff = Duration::from_secs(1);
    loop {
        match operation() {
            Ok(value) => return value,
            Err(error) => {
                tracing::warn!(
                    "Failed to {}: {}. Retrying in {:?}...",
                    what,
                    error,
                    backoff
                );
                thread::sleep(backoff);
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
            sleeps.lock().unwrap().push(d)
        });

        assert!(result.is_ok());
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

        let result = handle_with_retry(&handler, "topic", b"", &policy(), |_| {});

        assert_eq!("failure 3", result.unwrap_err().to_string());
        assert_eq!(3, handler.calls.load(Ordering::SeqCst));
    }
}
