use crate::mappers::{product_created_to_avro, product_deleted_to_avro, product_updated_to_avro};
use application::ports::out::publishers::ProductMessagePublisher;
use domain::events::{ProductCreatedEvent, ProductDeletedEvent, ProductUpdatedEvent};
use shared::contracts::AvroMessage;
use shared::contracts::topics::{PRODUCT_CREATED, PRODUCT_DELETED, PRODUCT_UPDATED};
use shared::domain::value_objects::ProductId;
use shared::kafka::KafkaProducer;
use std::sync::Mutex;

/// Topics this publisher writes to, for building its [`KafkaProducer`].
pub const PRODUCT_TOPICS: [&str; 3] = [PRODUCT_CREATED, PRODUCT_UPDATED, PRODUCT_DELETED];

/// Publishes product events straight to Kafka, keyed by product id.
///
/// The use cases call it from actix's blocking pool (`shared::web::blocking`), so waiting on
/// the mutex or the broker does not stall the async workers.
pub struct ProductKafkaMessagePublisher {
    producer: Mutex<KafkaProducer>,
}

impl ProductKafkaMessagePublisher {
    pub fn new(producer: KafkaProducer) -> Self {
        Self {
            producer: Mutex::new(producer),
        }
    }

    fn publish(
        &self,
        event_name: &str,
        topic: &str,
        product_id: ProductId,
        message: &impl AvroMessage,
    ) -> anyhow::Result<()> {
        tracing::info!("Received {event_name} for product with id: {product_id}");

        let result = message.to_avro_bytes().and_then(|payload| {
            self.producer
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .send(topic, &product_id.to_string(), &payload)
        });

        match &result {
            Ok(()) => tracing::info!(
                "{event_name} published successfully for product with id: {product_id}"
            ),
            Err(error) => tracing::error!(
                "Error while sending {event_name} to kafka for product id: {product_id}. {error:#}"
            ),
        }

        result
    }
}

impl ProductMessagePublisher for ProductKafkaMessagePublisher {
    fn publish_created(&self, event: ProductCreatedEvent) -> anyhow::Result<()> {
        self.publish(
            "ProductCreatedEvent",
            PRODUCT_CREATED,
            event.product().id(),
            &product_created_to_avro(&event),
        )
    }

    fn publish_updated(&self, event: ProductUpdatedEvent) -> anyhow::Result<()> {
        self.publish(
            "ProductUpdatedEvent",
            PRODUCT_UPDATED,
            event.product().id(),
            &product_updated_to_avro(&event),
        )
    }

    fn publish_deleted(&self, event: ProductDeletedEvent) -> anyhow::Result<()> {
        self.publish(
            "ProductDeletedEvent",
            PRODUCT_DELETED,
            event.product_id(),
            &product_deleted_to_avro(&event),
        )
    }
}
