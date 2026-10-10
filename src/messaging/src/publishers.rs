use crate::mappers::{
    payment_created_event_to_avro, payment_response_from_payload, payment_updated_event_to_avro,
};
use application::outbox::{OutboxMessageType, PaymentOutboxMessage, PaymentResponsePayload};
use application::ports::out::publishers::PaymentMessagePublisher;
use domain::events::{PaymentCreatedEvent, PaymentUpdatedEvent};
use shared::contracts::AvroMessage;
use shared::contracts::topics;
use shared::kafka::KafkaProducer;
use shared::outbox::ports::OutboxMessagePublisher;
use std::sync::Mutex;

/// Publishes `payment-created` / `payment-updated`, keyed by payment id.
pub struct PaymentKafkaPublisher {
    producer: Mutex<KafkaProducer>,
}

impl PaymentKafkaPublisher {
    pub fn new(hosts: Vec<String>) -> Self {
        Self {
            producer: Mutex::new(KafkaProducer::new(
                hosts,
                &[topics::PAYMENT_CREATED, topics::PAYMENT_UPDATED],
            )),
        }
    }

    fn send(&self, topic: &str, key: &str, bytes: &[u8]) -> anyhow::Result<()> {
        self.producer
            .lock()
            .map_err(|_| anyhow::anyhow!("Kafka producer lock is poisoned"))?
            .send(topic, key, bytes)
    }

    fn publish(&self, name: &str, topic: &str, key: String, bytes: Vec<u8>) -> anyhow::Result<()> {
        tracing::info!("Received {} for payment with id: {}", name, key);

        match self.send(topic, &key, &bytes) {
            Ok(()) => {
                tracing::info!(
                    "{} published successfully for payment with id: {}",
                    name,
                    key
                );
                Ok(())
            }
            Err(error) => {
                tracing::error!(
                    "Error while sending {} to kafka for payment id: {}. {}",
                    name,
                    key,
                    error,
                );
                Err(error)
            }
        }
    }
}

impl PaymentMessagePublisher for PaymentKafkaPublisher {
    fn publish_created(&self, event: PaymentCreatedEvent) -> anyhow::Result<()> {
        let bytes = payment_created_event_to_avro(&event).to_avro_bytes()?;

        self.publish(
            "PaymentCreatedEvent",
            topics::PAYMENT_CREATED,
            event.payment().id().to_string(),
            bytes,
        )
    }

    fn publish_updated(&self, event: PaymentUpdatedEvent) -> anyhow::Result<()> {
        let bytes = payment_updated_event_to_avro(&event).to_avro_bytes()?;

        self.publish(
            "PaymentUpdatedEvent",
            topics::PAYMENT_UPDATED,
            event.payment().id().to_string(),
            bytes,
        )
    }
}

/// Converts an outbox message into its topic, key (order id) and Avro payload.
pub fn encode(message: &PaymentOutboxMessage) -> anyhow::Result<(&'static str, String, Vec<u8>)> {
    match message.message_type() {
        OutboxMessageType::PaymentResponse => {
            let payload: PaymentResponsePayload = message.payload_as()?;
            let avro_model = payment_response_from_payload(message.saga_id(), payload);

            Ok((
                topics::PAYMENT_RESPONSE,
                avro_model.order_id.clone(),
                avro_model.to_avro_bytes()?,
            ))
        }
    }
}

/// Publishes outbox messages to Kafka, used by the outbox relay.
pub struct PaymentOutboxKafkaPublisher {
    producer: Mutex<KafkaProducer>,
}

impl PaymentOutboxKafkaPublisher {
    pub fn new(hosts: Vec<String>) -> Self {
        Self {
            producer: Mutex::new(KafkaProducer::new(hosts, &[topics::PAYMENT_RESPONSE])),
        }
    }
}

impl OutboxMessagePublisher<OutboxMessageType> for PaymentOutboxKafkaPublisher {
    fn publish(&self, message: &PaymentOutboxMessage) -> anyhow::Result<()> {
        let (topic, key, bytes) = encode(message)?;

        self.producer
            .lock()
            .map_err(|_| anyhow::anyhow!("Kafka producer lock is poisoned"))?
            .send(topic, &key, &bytes)
    }
}
