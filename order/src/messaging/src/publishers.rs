use crate::mappers::{
    payment_request_to_avro, restaurant_approval_cancel_request_to_avro,
    restaurant_approval_request_to_avro,
};
use application::outbox::{
    OrderApprovalCancelEventPayload, OrderApprovalEventPayload, OrderOutboxMessage,
    OrderPaymentEventPayload, OutboxMessageType,
};
use shared::contracts::AvroMessage;
use shared::contracts::topics;
use shared::kafka::KafkaProducer;
use shared::outbox::MessageKind;
use shared::outbox::ports::OutboxMessagePublisher;
use std::sync::Mutex;

/// An Avro-encoded record ready to be sent to Kafka.
#[derive(Debug, PartialEq)]
pub struct EncodedOutboxMessage {
    pub topic: &'static str,
    /// The order id, so all messages of one order keep their order.
    pub key: String,
    pub payload: Vec<u8>,
}

/// Maps an outbox message to its topic and Avro wire format.
pub fn encode(message: &OrderOutboxMessage) -> anyhow::Result<EncodedOutboxMessage> {
    match message.message_type() {
        OutboxMessageType::RestaurantApprovalRequest => {
            let payload: OrderApprovalEventPayload = message.payload_as()?;
            let key = payload.order_id.to_string();

            Ok(EncodedOutboxMessage {
                topic: topics::RESTAURANT_APPROVAL_REQUEST,
                key,
                payload: restaurant_approval_request_to_avro(payload).to_avro_bytes()?,
            })
        }
        OutboxMessageType::PaymentRequest => {
            let payload: OrderPaymentEventPayload = message.payload_as()?;
            let key = payload.order_id.to_string();

            Ok(EncodedOutboxMessage {
                topic: topics::PAYMENT_REQUEST,
                key,
                payload: payment_request_to_avro(payload).to_avro_bytes()?,
            })
        }
        OutboxMessageType::RestaurantApprovalCancelRequest => {
            let payload: OrderApprovalCancelEventPayload = message.payload_as()?;
            let key = payload.order_id.to_string();

            Ok(EncodedOutboxMessage {
                topic: topics::RESTAURANT_APPROVAL_CANCEL_REQUEST,
                key,
                payload: restaurant_approval_cancel_request_to_avro(payload).to_avro_bytes()?,
            })
        }
    }
}

/// Publishes the outbox messages to Kafka (used by the outbox relay).
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

impl OutboxMessagePublisher<OutboxMessageType> for OutboxKafkaPublisher {
    fn publish(&self, message: &OrderOutboxMessage) -> anyhow::Result<()> {
        let encoded = encode(message)?;

        let mut producer = self
            .producer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        producer.send(encoded.topic, &encoded.key, &encoded.payload)?;

        tracing::info!(
            "Published {} for saga {} to topic {}",
            message.message_type().as_str(),
            message.saga_id(),
            encoded.topic,
        );

        Ok(())
    }
}
