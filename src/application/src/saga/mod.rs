pub mod dtos;
pub mod payment_saga;
#[cfg(test)]
mod payment_saga_tests;
pub mod restaurant_approval_saga;
#[cfg(test)]
mod restaurant_approval_saga_tests;

use crate::outbox::OrderOutboxMessage;
use crate::ports::out::repositories::OrderRepository;
use crate::ports::out::saga_store::OrderSagaRepository;
use domain::aggregates::order::Order;
use domain::saga::{OrderSaga, SagaStatus};
use shared::domain::value_objects::OrderId;
use shared::error::{AppError, AppResult};
use uuid::Uuid;

/// Loads the saga and its order when the saga is still waiting in `expected_status`.
///
/// Returns `None` for unknown sagas and for replies that were already processed, which makes
/// every saga step idempotent towards Kafka redeliveries.
fn load_pending_step(
    saga_repository: &dyn OrderSagaRepository,
    order_repository: &dyn OrderRepository,
    saga_id: Uuid,
    order_id: Uuid,
    expected_status: SagaStatus,
) -> AppResult<Option<(OrderSaga, Order)>> {
    let Some(saga) = saga_repository.find_by_id(saga_id)? else {
        tracing::warn!("Saga {} not found, ignoring message", saga_id);
        return Ok(None);
    };

    if saga.status() != expected_status {
        tracing::info!(
            "Saga {} is already in status {}, ignoring duplicate message",
            saga_id,
            saga.status().as_str(),
        );
        return Ok(None);
    }

    if *saga.order_id().as_uuid() != order_id {
        return Err(AppError::Internal(anyhow::anyhow!(
            "Saga {} belongs to order {}, but the message references order {}",
            saga_id,
            saga.order_id().as_uuid(),
            order_id,
        )));
    }

    let order = order_repository
        .find_by_id(OrderId::from_uuid(order_id))?
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Order {} of saga {} not found",
                order_id,
                saga_id,
            ))
        })?;

    Ok(Some((saga, order)))
}

/// Stores the saga step. A saga that moved on between loading and storing it (a concurrent
/// duplicate of the same reply) is not an error: the other delivery already did the work.
fn store_step(
    saga_repository: &dyn OrderSagaRepository,
    order: &Order,
    saga: &OrderSaga,
    expected_status: SagaStatus,
    message: Option<OrderOutboxMessage>,
) -> AppResult<bool> {
    let advanced = saga_repository.advance(order, saga, expected_status, message)?;

    if !advanced {
        tracing::info!(
            "Saga {} is no longer in status {}, ignoring duplicate message",
            saga.id(),
            expected_status.as_str(),
        );
    }

    Ok(advanced)
}
