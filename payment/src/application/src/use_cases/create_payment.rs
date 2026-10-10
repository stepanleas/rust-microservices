use crate::commands::CreatePaymentCommand;
use crate::dtos::PaymentDto;
use crate::ports::out::publishers::PaymentMessagePublisher;
use crate::ports::out::repositories::PaymentRepository;
use domain::aggregates::payment::{Payment, PaymentId};
use domain::enums::PaymentStatus;
use domain::error::DomainError;
use domain::events::PaymentCreatedEvent;
use shared::domain::value_objects::{CustomerId, OrderId};
use shared::error::AppResult;
use std::sync::Arc;

/// Creates a payment manually (REST API) and publishes `payment-created`.
pub struct CreatePayment {
    repository: Arc<dyn PaymentRepository>,
    message_publisher: Arc<dyn PaymentMessagePublisher>,
}

impl CreatePayment {
    pub fn new(
        repository: Arc<dyn PaymentRepository>,
        message_publisher: Arc<dyn PaymentMessagePublisher>,
    ) -> Self {
        Self {
            repository,
            message_publisher,
        }
    }

    pub fn execute(&self, command: CreatePaymentCommand) -> AppResult<PaymentDto> {
        let order_id = OrderId::from_uuid(command.order_id());

        if self.repository.find_by_order_id(order_id)?.is_some() {
            return Err(DomainError::Conflict {
                message: format!(
                    "Payment for order with id: {} already exists",
                    command.order_id()
                ),
            }
            .into());
        }

        let payment = Payment::builder()
            .id(PaymentId::new())
            .order_id(order_id)
            .customer_id(CustomerId::from_uuid(command.customer_id()))
            .price(command.price())
            .payment_status(PaymentStatus::default())
            .build();

        let saved_payment = self.repository.save(payment)?;
        tracing::info!("Payment with id: {} created", saved_payment.id());

        let payment_dto = PaymentDto::from(&saved_payment);

        self.message_publisher
            .publish_created(PaymentCreatedEvent::new(saved_payment))?;

        Ok(payment_dto)
    }
}
