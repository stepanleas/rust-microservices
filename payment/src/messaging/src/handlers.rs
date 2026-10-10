use crate::mappers::payment_request_to_command;
use application::ports::r#in::messages::PaymentRequestMessageListener;
use shared::contracts::AvroMessage;
use shared::contracts::payment::PaymentRequestAvroModel;
use shared::contracts::topics;
use shared::kafka::MessageHandler;
use std::sync::Arc;

/// Entry point of every message this service consumes. Errors are propagated so the
/// listener retries the message and finally moves it to the dead letter topic.
pub struct PaymentMessageHandler {
    payment_request_listener: Arc<dyn PaymentRequestMessageListener>,
}

impl PaymentMessageHandler {
    pub fn new(payment_request_listener: Arc<dyn PaymentRequestMessageListener>) -> Self {
        Self {
            payment_request_listener,
        }
    }

    fn handle_payment_request(&self, payload: &[u8]) -> anyhow::Result<()> {
        for event in PaymentRequestAvroModel::from_avro_bytes(payload)? {
            tracing::info!(
                "Received PaymentRequest: id={}, saga_id={}, order_id={}, price={}, created_at={}",
                event.id,
                event.saga_id,
                event.order_id,
                event.price,
                event.created_at,
            );

            let command = payment_request_to_command(&event)?;
            self.payment_request_listener.payment_requested(command)?;
        }

        Ok(())
    }
}

impl MessageHandler for PaymentMessageHandler {
    fn topics(&self) -> Vec<&'static str> {
        vec![topics::PAYMENT_REQUEST]
    }

    fn handle(&self, topic: &str, payload: &[u8]) -> anyhow::Result<()> {
        match topic {
            topics::PAYMENT_REQUEST => self.handle_payment_request(payload),
            other => Err(anyhow::anyhow!(
                "Topic {other} is not consumed by this service"
            )),
        }
    }
}
