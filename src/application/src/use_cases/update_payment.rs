use crate::commands::UpdatePaymentCommand;
use crate::dtos::PaymentDto;
use crate::ports::out::publishers::PaymentMessagePublisher;
use crate::ports::out::repositories::PaymentRepository;
use domain::aggregates::payment::{Payment, PaymentId};
use domain::events::PaymentUpdatedEvent;
use shared::domain::value_objects::{CustomerId, OrderId};
use shared::error::{AppError, AppResult};
use std::sync::Arc;

/// Replaces an existing payment (REST API) and publishes `payment-updated`.
pub struct UpdatePayment {
    repository: Arc<dyn PaymentRepository>,
    message_publisher: Arc<dyn PaymentMessagePublisher>,
}

impl UpdatePayment {
    pub fn new(
        repository: Arc<dyn PaymentRepository>,
        message_publisher: Arc<dyn PaymentMessagePublisher>,
    ) -> Self {
        Self {
            repository,
            message_publisher,
        }
    }

    pub fn execute(&self, command: UpdatePaymentCommand) -> AppResult<PaymentDto> {
        let payment_id = PaymentId::from_uuid(command.id());
        let existing_payment = self.repository.find_by_id(payment_id)?.ok_or_else(|| {
            AppError::not_found(format!("Could not find payment with id: {payment_id}"))
        })?;

        let payment = Payment::builder()
            .id(existing_payment.id())
            .order_id(OrderId::from_uuid(command.order_id()))
            .customer_id(CustomerId::from_uuid(command.customer_id()))
            .price(command.price())
            .payment_status(command.payment_status())
            .failure_messages(existing_payment.failure_messages().to_vec())
            .build();

        let saved_payment = self.repository.save(payment)?;
        tracing::info!("Payment with id: {} updated", saved_payment.id());

        let payment_dto = PaymentDto::from(&saved_payment);

        self.message_publisher
            .publish_updated(PaymentUpdatedEvent::new(saved_payment))?;

        Ok(payment_dto)
    }
}
