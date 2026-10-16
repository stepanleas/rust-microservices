use crate::trace_context::{self, TRACEPARENT};
use rdkafka::ClientConfig;
use rdkafka::message::{Header, OwnedHeaders};
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;

/// How long librdkafka keeps retrying a message (broker down, leader election) before the send
/// fails. Short enough that the outbox relay notices and retries the row on its next poll.
const MESSAGE_TIMEOUT: Duration = Duration::from_secs(10);

/// Idempotent Kafka producer (`acks=all`) whose `send` waits for the broker's acknowledgement,
/// so the outbox relay marks a message completed only once Kafka has it.
///
/// Connects lazily: creating it never fails, and topics missing at startup are auto-created on
/// the first send (when the broker allows it).
pub struct KafkaProducer {
    hosts: Vec<String>,
    producer: Option<FutureProducer>,
}

impl KafkaProducer {
    pub fn new(hosts: Vec<String>) -> Self {
        Self {
            hosts,
            producer: None,
        }
    }

    fn producer(&mut self) -> anyhow::Result<&FutureProducer> {
        if self.producer.is_none() {
            let producer = ClientConfig::new()
                .set("bootstrap.servers", self.hosts.join(","))
                .set("acks", "all")
                .set("enable.idempotence", "true")
                .set("linger.ms", "5")
                .set("compression.type", "lz4")
                .set(
                    "message.timeout.ms",
                    MESSAGE_TIMEOUT.as_millis().to_string(),
                )
                .create()?;
            self.producer = Some(producer);
        }

        Ok(self.producer.as_ref().expect("producer was just created"))
    }

    /// Sends a keyed record. Messages with the same key (e.g. one order) keep their order.
    /// Carries the current trace (`traceparent` header), so the consumer continues it.
    pub fn send(&mut self, topic: &str, key: &str, payload: &[u8]) -> anyhow::Result<()> {
        match trace_context::current() {
            Some(traceparent) => self.send_with_headers(
                topic,
                key,
                payload,
                &[(TRACEPARENT, traceparent.as_bytes())],
            ),
            None => self.send_with_headers(topic, key, payload, &[]),
        }
    }

    /// Like [`Self::send`], with record headers (e.g. the W3C `traceparent`).
    pub fn send_with_headers(
        &mut self,
        topic: &str,
        key: &str,
        payload: &[u8],
        headers: &[(&str, &[u8])],
    ) -> anyhow::Result<()> {
        let mut record = FutureRecord::to(topic).key(key).payload(payload);
        if !headers.is_empty() {
            let headers = headers
                .iter()
                .fold(OwnedHeaders::new(), |all, (key, value)| {
                    all.insert(Header {
                        key,
                        value: Some(*value),
                    })
                });
            record = record.headers(headers);
        }

        let delivery = self
            .producer()?
            .send_result(record)
            .map_err(|(error, _)| error)?;

        match futures::executor::block_on(delivery) {
            Ok(Ok(_)) => Ok(()),
            Ok(Err((error, _))) => Err(anyhow::anyhow!("Failed to deliver to {topic}: {error}")),
            Err(_) => Err(anyhow::anyhow!(
                "Delivery to {topic} was cancelled (producer dropped)"
            )),
        }
    }
}
