use crate::commands::ProcessPaymentCommand;
use crate::dtos::PaymentDto;
use crate::outbox::{OutboxMessageType, PaymentResponsePayload};
use crate::ports::out::repositories::PaymentRepository;
use domain::aggregates::payment::{Payment, PaymentId};
use shared::domain::value_objects::{CustomerId, OrderId};
use shared::error::AppResult;
use shared::outbox::OutboxMessage;
use std::sync::Arc;

/// Handles the `payment-request` step of the order saga.
///
/// The payment is "made" by inserting a record in the local `payments` table
/// (there is no real payment provider). The payment and the `PAYMENT_RESPONSE`
/// outbox message are written in a single transaction, so the response is
/// eventually published even if Kafka is unavailable right now.
pub struct ProcessPayment {
    repository: Arc<dyn PaymentRepository>,
}

impl ProcessPayment {
    pub fn new(repository: Arc<dyn PaymentRepository>) -> Self {
        Self { repository }
    }

    /// Returns `None` when the order was already paid (duplicate delivery).
    pub fn execute(&self, command: ProcessPaymentCommand) -> AppResult<Option<PaymentDto>> {
        let order_id = OrderId::from_uuid(command.order_id());

        if let Some(existing_payment) = self.repository.find_by_order_id(order_id)? {
            tracing::warn!(
                "Payment {} already exists for order {}, skipping duplicate payment request for saga {}",
                existing_payment.id(),
                command.order_id(),
                command.saga_id(),
            );
            return Ok(None);
        }

        let mut payment = Payment::builder()
            .id(PaymentId::new())
            .order_id(order_id)
            .customer_id(CustomerId::from_uuid(command.customer_id()))
            .price(command.price())
            .build();

        payment.process();

        let payload = PaymentResponsePayload::from_payment(command.saga_id(), &payment);
        let message = OutboxMessage::new(
            command.saga_id(),
            OutboxMessageType::PaymentResponse,
            &payload,
        )?;

        let saved_payment = self.repository.save_with_outbox(payment, message)?;

        Ok(Some(PaymentDto::from(&saved_payment)))
    }
}
