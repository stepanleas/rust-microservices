use crate::mappers::{customer_created_to_avro, customer_updated_to_avro};
use application::ports::out::publishers::CustomerMessagePublisher;
use domain::events::{CustomerCreatedEvent, CustomerUpdatedEvent};
use shared::contracts::AvroMessage;
use shared::contracts::topics::{CUSTOMER_CREATED, CUSTOMER_UPDATED};
use shared::domain::value_objects::CustomerId;
use shared::kafka::KafkaProducer;
use std::sync::Mutex;

/// Publishes customer events straight to Kafka, keyed by customer id.
///
/// The use cases call it from actix's blocking pool (`shared::web::blocking`), so waiting on
/// the mutex or the broker does not stall the async workers.
pub struct CustomerKafkaPublisher {
    producer: Mutex<KafkaProducer>,
}

impl CustomerKafkaPublisher {
    pub fn new(producer: KafkaProducer) -> Self {
        Self {
            producer: Mutex::new(producer),
        }
    }

    fn publish(
        &self,
        event_name: &str,
        topic: &str,
        customer_id: CustomerId,
        message: &impl AvroMessage,
    ) -> anyhow::Result<()> {
        tracing::info!("Received {event_name} for customer with id: {customer_id}");

        let result = message.to_avro_bytes().and_then(|payload| {
            self.producer
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .send(topic, &customer_id.to_string(), &payload)
        });

        match &result {
            Ok(()) => tracing::info!(
                "{event_name} published successfully for customer with id: {customer_id}"
            ),
            Err(error) => tracing::error!(
                "Error while sending {event_name} to kafka for customer id: {customer_id}. {error:#}"
            ),
        }

        result
    }
}

impl CustomerMessagePublisher for CustomerKafkaPublisher {
    fn publish_created(&self, event: CustomerCreatedEvent) -> anyhow::Result<()> {
        self.publish(
            "CustomerCreatedEvent",
            CUSTOMER_CREATED,
            event.customer().id(),
            &customer_created_to_avro(&event),
        )
    }

    fn publish_updated(&self, event: CustomerUpdatedEvent) -> anyhow::Result<()> {
        self.publish(
            "CustomerUpdatedEvent",
            CUSTOMER_UPDATED,
            event.customer().id(),
            &customer_updated_to_avro(&event),
        )
    }
}
