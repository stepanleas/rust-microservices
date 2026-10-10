use rdkafka::ClientConfig;
use rdkafka::consumer::{BaseConsumer, CommitMode, Consumer};
use rdkafka::message::{Headers, Message};
use std::time::Duration;

/// A record read from Kafka, copied out of librdkafka's buffer.
#[derive(Debug, Clone)]
pub struct KafkaMessage {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub key: Vec<u8>,
    pub payload: Vec<u8>,
    pub headers: Vec<(String, Vec<u8>)>,
}

impl KafkaMessage {
    pub fn header(&self, name: &str) -> Option<&[u8]> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_slice())
    }
}

/// Member of a consumer group: Kafka spreads the partitions of the subscribed topics over every
/// consumer with the same group id (cooperative-sticky rebalancing), so N replicas of a service
/// share the load instead of each reading everything.
///
/// At-least-once: an offset is committed (in the background, every second, and on rebalance)
/// only after it was passed to [`Self::store_offset`], i.e. once the message was handled.
pub struct KafkaConsumer {
    consumer: BaseConsumer,
}

impl KafkaConsumer {
    pub fn new(hosts: Vec<String>, group_id: String, topics: &[&str]) -> anyhow::Result<Self> {
        let consumer: BaseConsumer = ClientConfig::new()
            .set("bootstrap.servers", hosts.join(","))
            .set("group.id", group_id)
            .set("partition.assignment.strategy", "cooperative-sticky")
            .set("auto.offset.reset", "earliest")
            .set("enable.auto.commit", "true")
            .set("auto.commit.interval.ms", "1000")
            .set("enable.auto.offset.store", "false")
            // Subscribing to a topic nobody produced to yet creates it (if the broker allows it).
            .set("allow.auto.create.topics", "true")
            .create()?;
        consumer.subscribe(topics)?;

        Ok(Self { consumer })
    }

    /// Waits up to `timeout` for the next message. `Ok(None)` when none arrived in time.
    pub fn poll(&self, timeout: Duration) -> anyhow::Result<Option<KafkaMessage>> {
        let Some(result) = self.consumer.poll(timeout) else {
            return Ok(None);
        };
        let message = result?;

        let headers = message
            .headers()
            .map(|headers| {
                headers
                    .iter()
                    .map(|header| {
                        (
                            header.key.to_string(),
                            header.value.unwrap_or_default().to_vec(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(Some(KafkaMessage {
            topic: message.topic().to_string(),
            partition: message.partition(),
            offset: message.offset(),
            key: message.key().unwrap_or_default().to_vec(),
            payload: message.payload().unwrap_or_default().to_vec(),
            headers,
        }))
    }

    /// Marks the message (and every earlier one of its partition) as processed.
    pub fn store_offset(&self, message: &KafkaMessage) -> anyhow::Result<()> {
        // librdkafka commits `offset + 1`, the next message to read.
        self.consumer
            .store_offset(&message.topic, message.partition, message.offset)?;
        Ok(())
    }

    /// The `(topic, partition)` pairs currently assigned to this consumer by the group.
    pub fn assignment(&self) -> anyhow::Result<Vec<(String, i32)>> {
        Ok(self
            .consumer
            .assignment()?
            .elements()
            .iter()
            .map(|element| (element.topic().to_string(), element.partition()))
            .collect())
    }

    /// Commits the stored offsets now (e.g. before shutting down).
    pub fn commit(&self) -> anyhow::Result<()> {
        self.consumer.commit_consumer_state(CommitMode::Sync)?;
        Ok(())
    }
}
