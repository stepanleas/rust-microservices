use crate::mappers::{
    customer_from_avro, parse_product_id, payment_response_from_avro, product_from_avro,
    restaurant_approval_response_from_avro,
};
use application::ports::r#in::messages::{
    CustomerMessageListener, PaymentResponseMessageListener, ProductMessageListener,
    RestaurantApprovalResponseMessageListener,
};
use application::saga::dtos::{OrderApprovalStatus, PaymentResponseStatus};
use shared::contracts::AvroMessage;
use shared::contracts::catalog::{
    ProductCreatedEventAvroModel, ProductDeletedEventAvroModel, ProductUpdatedEventAvroModel,
};
use shared::contracts::customer::{CustomerCreatedEventAvroModel, CustomerUpdatedEventAvroModel};
use shared::contracts::payment::PaymentResponseAvroModel;
use shared::contracts::restaurant::RestaurantApprovalResponseAvroModel;
use shared::contracts::topics;
use shared::kafka::MessageHandler;
use std::sync::Arc;

/// Entry point of every message the order service consumes; dispatches by topic.
///
/// Errors are propagated so the listener retries the message and finally dead-letters it.
/// Every listener is idempotent (upserts, and saga replies for a saga that moved on are
/// ignored), so redelivered messages are harmless.
pub struct OrderMessageHandler {
    customer_listener: Arc<dyn CustomerMessageListener>,
    product_listener: Arc<dyn ProductMessageListener>,
    restaurant_approval_listener: Arc<dyn RestaurantApprovalResponseMessageListener>,
    payment_listener: Arc<dyn PaymentResponseMessageListener>,
}

impl OrderMessageHandler {
    pub fn new(
        customer_listener: Arc<dyn CustomerMessageListener>,
        product_listener: Arc<dyn ProductMessageListener>,
        restaurant_approval_listener: Arc<dyn RestaurantApprovalResponseMessageListener>,
        payment_listener: Arc<dyn PaymentResponseMessageListener>,
    ) -> Self {
        Self {
            customer_listener,
            product_listener,
            restaurant_approval_listener,
            payment_listener,
        }
    }

    fn customer_created(&self, payload: &[u8]) -> anyhow::Result<()> {
        for event in CustomerCreatedEventAvroModel::from_avro_bytes(payload)? {
            tracing::info!(
                "Received CustomerCreatedEvent: id={}, customer_id={}",
                event.id,
                event.customer.id,
            );
            let customer = customer_from_avro(&event.customer)?;
            self.customer_listener.customer_created(customer)?;
        }

        Ok(())
    }

    fn customer_updated(&self, payload: &[u8]) -> anyhow::Result<()> {
        for event in CustomerUpdatedEventAvroModel::from_avro_bytes(payload)? {
            tracing::info!(
                "Received CustomerUpdatedEvent: id={}, customer_id={}",
                event.id,
                event.customer.id,
            );
            let customer = customer_from_avro(&event.customer)?;
            self.customer_listener.customer_updated(customer)?;
        }

        Ok(())
    }

    fn product_created(&self, payload: &[u8]) -> anyhow::Result<()> {
        for event in ProductCreatedEventAvroModel::from_avro_bytes(payload)? {
            tracing::info!(
                "Received ProductCreatedEvent: id={}, product_id={}",
                event.id,
                event.product.id,
            );
            let product = product_from_avro(&event.product)?;
            self.product_listener.product_created(product)?;
        }

        Ok(())
    }

    fn product_updated(&self, payload: &[u8]) -> anyhow::Result<()> {
        for event in ProductUpdatedEventAvroModel::from_avro_bytes(payload)? {
            tracing::info!(
                "Received ProductUpdatedEvent: id={}, product_id={}",
                event.id,
                event.product.id,
            );
            let product = product_from_avro(&event.product)?;
            self.product_listener.product_updated(product)?;
        }

        Ok(())
    }

    fn product_deleted(&self, payload: &[u8]) -> anyhow::Result<()> {
        for event in ProductDeletedEventAvroModel::from_avro_bytes(payload)? {
            tracing::info!(
                "Received ProductDeletedEvent: id={}, product_id={}",
                event.id,
                event.product_id,
            );
            let product_id = parse_product_id(&event.product_id)?;
            self.product_listener.product_deleted(product_id)?;
        }

        Ok(())
    }

    fn restaurant_approval_response(&self, payload: &[u8]) -> anyhow::Result<()> {
        for event in RestaurantApprovalResponseAvroModel::from_avro_bytes(payload)? {
            tracing::info!(
                "Received RestaurantApprovalResponse: id={}, saga_id={}, order_id={}, status={}",
                event.id,
                event.saga_id,
                event.order_id,
                event.order_approval_status,
            );
            let response = restaurant_approval_response_from_avro(event)?;

            match response.status {
                OrderApprovalStatus::Approved => {
                    self.restaurant_approval_listener.order_approved(response)?
                }
                OrderApprovalStatus::Rejected => {
                    self.restaurant_approval_listener.order_rejected(response)?
                }
            }
        }

        Ok(())
    }

    fn payment_response(&self, payload: &[u8]) -> anyhow::Result<()> {
        for event in PaymentResponseAvroModel::from_avro_bytes(payload)? {
            tracing::info!(
                "Received PaymentResponse: id={}, saga_id={}, order_id={}, status={}",
                event.id,
                event.saga_id,
                event.order_id,
                event.payment_status,
            );
            let response = payment_response_from_avro(event)?;

            match response.status {
                PaymentResponseStatus::Completed => {
                    self.payment_listener.payment_completed(response)?
                }
                PaymentResponseStatus::Failed | PaymentResponseStatus::Cancelled => {
                    self.payment_listener.payment_failed(response)?
                }
            }
        }

        Ok(())
    }
}

impl MessageHandler for OrderMessageHandler {
    fn topics(&self) -> Vec<&'static str> {
        vec![
            topics::CUSTOMER_CREATED,
            topics::CUSTOMER_UPDATED,
            topics::PRODUCT_CREATED,
            topics::PRODUCT_UPDATED,
            topics::PRODUCT_DELETED,
            topics::RESTAURANT_APPROVAL_RESPONSE,
            topics::PAYMENT_RESPONSE,
        ]
    }

    fn handle(&self, topic: &str, payload: &[u8]) -> anyhow::Result<()> {
        match topic {
            topics::CUSTOMER_CREATED => self.customer_created(payload),
            topics::CUSTOMER_UPDATED => self.customer_updated(payload),
            topics::PRODUCT_CREATED => self.product_created(payload),
            topics::PRODUCT_UPDATED => self.product_updated(payload),
            topics::PRODUCT_DELETED => self.product_deleted(payload),
            topics::RESTAURANT_APPROVAL_RESPONSE => self.restaurant_approval_response(payload),
            topics::PAYMENT_RESPONSE => self.payment_response(payload),
            other => Err(anyhow::anyhow!("No handler for topic {other}")),
        }
    }
}
