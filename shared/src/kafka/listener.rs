use super::{KafkaConsumer, KafkaProducer, dead_letter_topic};
use crate::shutdown::Shutdown;
use crate::trace_context::{self, TRACEPARENT};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::field::Empty;

/// How long one poll waits for a message before polling again.
const POLL_TIMEOUT: Duration = Duration::from_millis(500);
/// How often the partition assignment of the consumer is published as a metric.
const ASSIGNMENT_INTERVAL: Duration = Duration::from_secs(10);

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
///
/// Each message is handled in a span that continues the producer's trace (`traceparent`
/// header). Metrics:
///
/// - `kafka_messages_handled_total{topic, result="ok"|"retry"|"dlq"}`: `retry` counts failed
///   attempts that were retried
/// - `kafka_handler_duration_seconds{topic}`: from the first attempt to the outcome (histogram)
/// - `kafka_consumer_assigned_partitions{topic}`: partitions this replica owns (gauge)
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
    let mut assignment_published: Option<Instant> = None;

    while !shutdown.is_triggered() {
        if assignment_published.is_none_or(|at| at.elapsed() >= ASSIGNMENT_INTERVAL) {
            assignment_published = Some(Instant::now());
            publish_assignment(&consumer, &topics);
        }

        let message = match consumer.poll(POLL_TIMEOUT) {
            Ok(Some(message)) => message,
            Ok(None) => continue,
            // librdkafka reports connection problems as events through poll and reconnects on
            // its own. Keep polling: errors queued while Kafka was down are drained quickly,
            // and the consumer group protocol only progresses while poll is called.
            Err(error) => {
                tracing::warn!("Kafka poll error: {}", error);
                continue;
            }
        };

        let span = tracing::info_span!(
            "kafka_message",
            topic = %message.topic,
            partition = message.partition,
            offset = message.offset,
            key = %String::from_utf8_lossy(&message.key),
            trace_id = Empty,
            otel.name = format!("{} process", message.topic),
            otel.kind = "consumer",
        );
        let traceparent = message
            .header(TRACEPARENT)
            .and_then(|value| std::str::from_utf8(value).ok());
        trace_context::continue_trace(&span, traceparent);
        let _span = span.entered();

        let started = Instant::now();
        let outcome = handle_with_retry(
            handler.as_ref(),
            &message.topic,
            &message.payload,
            &retry,
            |backoff| shutdown.sleep(backoff),
        );
        let result = match outcome {
            Outcome::Handled => Some("ok"),
            Outcome::GaveUp(_) => Some("dlq"),
            Outcome::Interrupted => None,
        };
        if let Some(result) = result {
            let topic = message.topic.clone();
            metrics::counter!("kafka_messages_handled_total", "topic" => topic.clone(), "result" => result)
                .increment(1);
            metrics::histogram!("kafka_handler_duration_seconds", "topic" => topic)
                .record(started.elapsed().as_secs_f64());
        }

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

/// Sets `kafka_consumer_assigned_partitions` for every subscribed topic (0 when none of its
/// partitions is assigned here), so the dashboards show how the group spread the partitions.
fn publish_assignment(consumer: &KafkaConsumer, topics: &[&'static str]) {
    let assignment = match consumer.assignment() {
        Ok(assignment) => assignment,
        Err(error) => {
            tracing::debug!("Failed to read the partition assignment: {error}");
            return;
        }
    };

    let mut partitions: HashMap<&str, usize> = topics.iter().map(|topic| (*topic, 0)).collect();
    for (topic, _) in &assignment {
        if let Some(count) = partitions.get_mut(topic.as_str()) {
            *count += 1;
        }
    }
    for (topic, count) in partitions {
        metrics::gauge!("kafka_consumer_assigned_partitions", "topic" => topic).set(count as f64);
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
                metrics::counter!(
                    "kafka_messages_handled_total",
                    "topic" => topic.to_string(),
                    "result" => "retry",
                )
                .increment(1);
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
