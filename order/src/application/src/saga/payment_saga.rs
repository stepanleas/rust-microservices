use crate::outbox::{OrderApprovalCancelEventPayload, OrderOutboxMessage, OutboxMessageType};
use crate::ports::out::repositories::OrderRepository;
use crate::ports::out::saga_store::OrderSagaRepository;
use crate::saga::dtos::PaymentResponse;
use crate::saga::{load_pending_step, record_saga_end, store_step};
use domain::saga::SagaStatus;
use shared::error::AppResult;
use std::sync::Arc;

/// Saga step 2 -> end: reacts to the payment reply.
///
/// * completed: order `APPROVED -> PAID`, saga `PROCESSING -> SUCCEEDED`.
/// * failed/cancelled: order `APPROVED -> CANCELLED`, saga `PROCESSING -> FAILED`, and a
///   restaurant approval cancel request is queued so the restaurant gives the stock back
///   (compensation of step 1).
pub struct PaymentSaga {
    saga_repository: Arc<dyn OrderSagaRepository>,
    order_repository: Arc<dyn OrderRepository>,
}

impl PaymentSaga {
    pub fn new(
        saga_repository: Arc<dyn OrderSagaRepository>,
        order_repository: Arc<dyn OrderRepository>,
    ) -> Self {
        Self {
            saga_repository,
            order_repository,
        }
    }

    pub fn process(&self, response: &PaymentResponse) -> AppResult<()> {
        let Some((mut saga, mut order)) = load_pending_step(
            self.saga_repository.as_ref(),
            self.order_repository.as_ref(),
            response.saga_id,
            response.order_id,
            SagaStatus::Processing,
        )?
        else {
            return Ok(());
        };

        order.pay()?;
        saga.payment_completed()?;

        if store_step(
            self.saga_repository.as_ref(),
            &order,
            &saga,
            SagaStatus::Processing,
            None,
        )? {
            record_saga_end(&saga, "paid");
            tracing::info!(
                "Order {} paid with payment {}",
                response.order_id,
                response.payment_id,
            );
        }

        Ok(())
    }

    /// Only runs while the saga is `PROCESSING`, i.e. after the restaurant approved the order,
    /// so the compensation is always due here.
    pub fn rollback(&self, response: &PaymentResponse) -> AppResult<()> {
        let Some((mut saga, mut order)) = load_pending_step(
            self.saga_repository.as_ref(),
            self.order_repository.as_ref(),
            response.saga_id,
            response.order_id,
            SagaStatus::Processing,
        )?
        else {
            return Ok(());
        };

        order.cancel(response.failure_messages.clone())?;
        saga.fail()?;

        let payload = OrderApprovalCancelEventPayload::from_order(&order, saga.id());
        let compensation = OrderOutboxMessage::new(
            saga.id(),
            OutboxMessageType::RestaurantApprovalCancelRequest,
            &payload,
        )?;

        if store_step(
            self.saga_repository.as_ref(),
            &order,
            &saga,
            SagaStatus::Processing,
            Some(compensation),
        )? {
            record_saga_end(&saga, "payment_failed");
            metrics::counter!("saga_compensations_total").increment(1);
            tracing::info!(
                "Payment failed for order {}: {:?}. Restaurant approval cancellation requested",
                response.order_id,
                response.failure_messages,
            );
        }

        Ok(())
    }
}
