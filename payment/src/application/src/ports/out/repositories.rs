use crate::outbox::PaymentOutboxMessage;
use domain::aggregates::payment::{Payment, PaymentId};
use shared::domain::value_objects::OrderId;

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait PaymentRepository: Send + Sync {
    fn save(&self, payment: Payment) -> anyhow::Result<Payment>;
    /// Inserts the payment and the outbox message atomically (transactional outbox).
    fn save_with_outbox(
        &self,
        payment: Payment,
        message: PaymentOutboxMessage,
    ) -> anyhow::Result<Payment>;
    fn find_by_id(&self, payment_id: PaymentId) -> anyhow::Result<Option<Payment>>;
    fn find_by_order_id(&self, order_id: OrderId) -> anyhow::Result<Option<Payment>>;
}
