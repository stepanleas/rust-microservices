use crate::outbox::{OrderOutboxMessage, OrderPaymentEventPayload, OutboxMessageType};
use crate::ports::out::repositories::OrderRepository;
use crate::ports::out::saga_store::OrderSagaRepository;
use crate::saga::dtos::RestaurantApprovalResponse;
use crate::saga::{load_pending_step, store_step};
use domain::saga::SagaStatus;
use shared::error::AppResult;
use std::sync::Arc;

/// Saga step 1 -> 2: reacts to the restaurant reply.
///
/// * approved: order `PENDING -> APPROVED`, saga `STARTED -> PROCESSING`, payment request queued.
/// * rejected: order `PENDING -> CANCELLED`, saga `STARTED -> FAILED`. Nothing was reserved by
///   the restaurant, so there is nothing to compensate.
pub struct RestaurantApprovalSaga {
    saga_repository: Arc<dyn OrderSagaRepository>,
    order_repository: Arc<dyn OrderRepository>,
}

impl RestaurantApprovalSaga {
    pub fn new(
        saga_repository: Arc<dyn OrderSagaRepository>,
        order_repository: Arc<dyn OrderRepository>,
    ) -> Self {
        Self {
            saga_repository,
            order_repository,
        }
    }

    pub fn process(&self, response: &RestaurantApprovalResponse) -> AppResult<()> {
        let Some((mut saga, mut order)) = load_pending_step(
            self.saga_repository.as_ref(),
            self.order_repository.as_ref(),
            response.saga_id,
            response.order_id,
            SagaStatus::Started,
        )?
        else {
            return Ok(());
        };

        order.approve()?;
        saga.restaurant_approved()?;

        let payload = OrderPaymentEventPayload::from_order(&order, saga.id());
        let message =
            OrderOutboxMessage::new(saga.id(), OutboxMessageType::PaymentRequest, &payload)?;

        if store_step(
            self.saga_repository.as_ref(),
            &order,
            &saga,
            SagaStatus::Started,
            Some(message),
        )? {
            tracing::info!(
                "Order {} approved by the restaurant, payment requested",
                response.order_id,
            );
        }

        Ok(())
    }

    pub fn rollback(&self, response: &RestaurantApprovalResponse) -> AppResult<()> {
        let Some((mut saga, mut order)) = load_pending_step(
            self.saga_repository.as_ref(),
            self.order_repository.as_ref(),
            response.saga_id,
            response.order_id,
            SagaStatus::Started,
        )?
        else {
            return Ok(());
        };

        order.cancel(response.failure_messages.clone())?;
        saga.fail()?;

        if store_step(
            self.saga_repository.as_ref(),
            &order,
            &saga,
            SagaStatus::Started,
            None,
        )? {
            tracing::info!(
                "Order {} rejected by the restaurant: {:?}",
                response.order_id,
                response.failure_messages,
            );
        }

        Ok(())
    }
}
