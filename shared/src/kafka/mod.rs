//! Kafka adapters on top of librdkafka (`rdkafka`): consumer groups, idempotent producer, headers.

mod admin;
mod consumer;
mod listener;
mod producer;

pub use admin::ensure_topics;
pub use consumer::{KafkaConsumer, KafkaMessage};
pub use listener::{MessageHandler, RetryPolicy, listen};
pub use producer::KafkaProducer;

/// Topic that receives the messages of `topic` a consumer gave up on.
pub fn dead_letter_topic(topic: &str) -> String {
    format!("{topic}.dlq")
}
