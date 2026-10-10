use crate::mappers::{ProductMessagingMapper, RestaurantApprovalMessagingMapper};
use application::ports::r#in::messages::{
    ProductMessageListener, RestaurantApprovalMessageListener,
};
use shared::contracts::AvroMessage;
use shared::contracts::catalog::{
    ProductCreatedEventAvroModel, ProductDeletedEventAvroModel, ProductUpdatedEventAvroModel,
};
use shared::contracts::restaurant::{
    RestaurantApprovalCancelRequestAvroModel, RestaurantApprovalRequestAvroModel,
};
use shared::contracts::topics;
use shared::kafka::MessageHandler;
use std::sync::Arc;

/// Every topic the restaurant service consumes. Errors are propagated, so the listener retries
/// the message and finally parks it in `<topic>.dlq`; the listeners are idempotent.
pub struct RestaurantMessageHandler {
    product_listener: Arc<dyn ProductMessageListener>,
    approval_listener: Arc<dyn RestaurantApprovalMessageListener>,
}

impl RestaurantMessageHandler {
    pub fn new(
        product_listener: Arc<dyn ProductMessageListener>,
        approval_listener: Arc<dyn RestaurantApprovalMessageListener>,
    ) -> Self {
        Self {
            product_listener,
            approval_listener,
        }
    }

    fn product_created(&self, payload: &[u8]) -> anyhow::Result<()> {
        for event in ProductCreatedEventAvroModel::from_avro_bytes(payload)? {
            tracing::info!(
                "Received ProductCreatedEvent: id={}, product_id={}",
                event.id,
                event.product.id,
            );
            let product = ProductMessagingMapper::product_avro_model_to_dto(&event.product)?;
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
            let product = ProductMessagingMapper::product_avro_model_to_dto(&event.product)?;
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
            let product_id = ProductMessagingMapper::product_id(&event.product_id)?;
            self.product_listener.product_deleted(product_id)?;
        }
        Ok(())
    }

    fn approval_requested(&self, payload: &[u8]) -> anyhow::Result<()> {
        for event in RestaurantApprovalRequestAvroModel::from_avro_bytes(payload)? {
            tracing::info!(
                "Received RestaurantApprovalRequest: id={}, saga_id={}, order_id={}, products={}",
                event.id,
                event.saga_id,
                event.order_id,
                event.products.len(),
            );
            let command = RestaurantApprovalMessagingMapper::approval_request_to_command(&event)?;
            self.approval_listener.approval_requested(command)?;
        }
        Ok(())
    }

    fn approval_cancel_requested(&self, payload: &[u8]) -> anyhow::Result<()> {
        for event in RestaurantApprovalCancelRequestAvroModel::from_avro_bytes(payload)? {
            tracing::info!(
                "Received RestaurantApprovalCancelRequest: id={}, saga_id={}, order_id={}, products={}",
                event.id,
                event.saga_id,
                event.order_id,
                event.products.len(),
            );
            let command =
                RestaurantApprovalMessagingMapper::approval_cancel_request_to_command(&event)?;
            self.approval_listener.approval_cancel_requested(command)?;
        }
        Ok(())
    }
}

impl MessageHandler for RestaurantMessageHandler {
    fn topics(&self) -> Vec<&'static str> {
        vec![
            topics::PRODUCT_CREATED,
            topics::PRODUCT_UPDATED,
            topics::PRODUCT_DELETED,
            topics::RESTAURANT_APPROVAL_REQUEST,
            topics::RESTAURANT_APPROVAL_CANCEL_REQUEST,
        ]
    }

    fn handle(&self, topic: &str, payload: &[u8]) -> anyhow::Result<()> {
        match topic {
            topics::PRODUCT_CREATED => self.product_created(payload),
            topics::PRODUCT_UPDATED => self.product_updated(payload),
            topics::PRODUCT_DELETED => self.product_deleted(payload),
            topics::RESTAURANT_APPROVAL_REQUEST => self.approval_requested(payload),
            topics::RESTAURANT_APPROVAL_CANCEL_REQUEST => self.approval_cancel_requested(payload),
            other => Err(anyhow::anyhow!("Unexpected topic: {other}")),
        }
    }
}
