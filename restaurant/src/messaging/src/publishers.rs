use application::outbox::{
    OutboxMessageType, RestaurantApprovalResponsePayload, RestaurantOutboxMessage,
};
use shared::contracts::AvroMessage;
use shared::contracts::restaurant::RestaurantApprovalResponseAvroModel;
use shared::contracts::topics;
use shared::kafka::KafkaProducer;
use shared::outbox::MessageKind;
use shared::outbox::ports::OutboxMessagePublisher;
use std::sync::Mutex;

/// Publishes outbox messages to Kafka as Avro records.
pub struct OutboxKafkaPublisher {
    producer: Mutex<KafkaProducer>,
}

impl OutboxKafkaPublisher {
    pub fn new(producer: KafkaProducer) -> Self {
        Self {
            producer: Mutex::new(producer),
        }
    }
}

/// Kafka record of an outbox message.
#[derive(Debug, Clone, PartialEq)]
pub struct KafkaRecord {
    pub topic: &'static str,
    /// The order id: saga messages are keyed by order.
    pub key: String,
    pub payload: Vec<u8>,
}

/// Maps an outbox message to the destination topic, record key and Avro bytes.
pub fn encode(message: &RestaurantOutboxMessage) -> anyhow::Result<KafkaRecord> {
    match message.message_type() {
        OutboxMessageType::RestaurantApprovalResponse => {
            let payload: RestaurantApprovalResponsePayload = message.payload_as()?;
            let avro_model = RestaurantApprovalResponseAvroModel {
                id: message.id().to_string(),
                saga_id: payload.saga_id.to_string(),
                order_id: payload.order_id.to_string(),
                order_approval_status: payload.order_approval_status,
                failure_messages: payload.failure_messages,
                created_at: payload.created_at,
            };

            Ok(KafkaRecord {
                topic: topics::RESTAURANT_APPROVAL_RESPONSE,
                key: avro_model.order_id.clone(),
                payload: avro_model.to_avro_bytes()?,
            })
        }
    }
}

impl OutboxMessagePublisher<OutboxMessageType> for OutboxKafkaPublisher {
    fn publish(&self, message: &RestaurantOutboxMessage) -> anyhow::Result<()> {
        let record = encode(message)?;

        let mut producer = self
            .producer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        producer.send(record.topic, &record.key, &record.payload)?;

        tracing::info!(
            "Published {} message {} for saga {} to topic {}",
            message.message_type().as_str(),
            message.id(),
            message.saga_id(),
            record.topic,
        );

        Ok(())
    }
}
