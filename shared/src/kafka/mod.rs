//! Kafka adapters on top of the `kafka` crate (Kafka 3.x protocol).

mod consumer;
mod listener;
mod producer;

pub use consumer::KafkaConsumer;
pub use listener::{MessageHandler, RetryPolicy, listen};
pub use producer::KafkaProducer;

/// Topic that receives the messages of `topic` a consumer gave up on.
pub fn dead_letter_topic(topic: &str) -> String {
    format!("{topic}.dlq")
}
