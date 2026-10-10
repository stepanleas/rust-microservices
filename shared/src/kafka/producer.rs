use kafka::client::{KafkaClient, RequiredAcks};
use kafka::producer::{Producer, Record};
use std::time::Duration;

/// Lazily connected Kafka producer that rebuilds itself after a failed send.
///
/// The `kafka` crate snapshots the partitions of every known topic when a `Producer` is created,
/// so a producer created before a topic existed (or before Kafka was reachable) can never send to
/// it. Dropping the producer on failure and recreating it with fresh metadata on the next send
/// makes the outbox relay recover on its next attempt.
pub struct KafkaProducer {
    hosts: Vec<String>,
    topics: Vec<String>,
    producer: Option<Producer>,
}

impl KafkaProducer {
    pub fn new(hosts: Vec<String>, topics: &[&str]) -> Self {
        Self {
            hosts,
            topics: topics.iter().map(|topic| topic.to_string()).collect(),
            producer: None,
        }
    }

    fn producer(&mut self) -> anyhow::Result<&mut Producer> {
        if self.producer.is_none() {
            let mut client = KafkaClient::new(self.hosts.clone());
            // Requesting explicit topics lets the broker auto-create missing ones.
            client.load_metadata(&self.topics)?;

            for topic in &self.topics {
                let has_partitions = client
                    .topics()
                    .partitions(topic)
                    .is_some_and(|partitions| !partitions.is_empty());

                if !has_partitions {
                    anyhow::bail!("Topic {topic} has no partitions yet");
                }
            }

            let producer = Producer::from_client(client)
                .with_ack_timeout(Duration::from_secs(1))
                .with_required_acks(RequiredAcks::One)
                .create()?;

            self.producer = Some(producer);
        }

        Ok(self.producer.as_mut().expect("producer was just created"))
    }

    /// Sends a keyed record. Messages with the same key (e.g. one order) keep their order.
    pub fn send(&mut self, topic: &str, key: &str, payload: &[u8]) -> anyhow::Result<()> {
        let record = Record::from_key_value(topic, key.as_bytes(), payload);

        if let Err(error) = self.producer()?.send(&record) {
            self.producer = None;
            return Err(error.into());
        }

        Ok(())
    }
}
