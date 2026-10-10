use crate::outbox::OrderOutboxMessage;
use domain::aggregates::order::Order;
use domain::saga::{OrderSaga, SagaStatus};
use uuid::Uuid;

/// Persists the order, its saga and the outbox messages atomically (one database transaction).
#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait OrderSagaRepository: Send + Sync {
    fn find_by_id(&self, saga_id: Uuid) -> anyhow::Result<Option<OrderSaga>>;

    /// Inserts a new order, starts its saga and stores the first outbox message.
    fn start(
        &self,
        order: &Order,
        saga: &OrderSaga,
        message: &OrderOutboxMessage,
    ) -> anyhow::Result<()>;

    /// Stores the new order and saga state, optionally adding the next outbox message.
    ///
    /// Returns `false` without changing anything when the stored saga is no longer in
    /// `expected_status`, i.e. somebody else (e.g. a concurrent duplicate of the same reply)
    /// already moved it on.
    fn advance(
        &self,
        order: &Order,
        saga: &OrderSaga,
        expected_status: SagaStatus,
        message: Option<OrderOutboxMessage>,
    ) -> anyhow::Result<bool>;
}
