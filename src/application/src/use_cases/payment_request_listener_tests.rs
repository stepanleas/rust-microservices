use crate::commands::ProcessPaymentCommand;
use crate::ports::r#in::messages::PaymentRequestMessageListener;
use crate::ports::out::repositories::MockPaymentRepository;
use crate::use_cases::{ApplicationPaymentRequestMessageListener, ProcessPayment};
use domain::aggregates::payment::{Payment, PaymentId};
use domain::enums::PaymentStatus;
use rust_decimal::Decimal;
use shared::domain::value_objects::{CustomerId, OrderId};
use std::sync::Arc;
use uuid::Uuid;

fn listener(repository: MockPaymentRepository) -> ApplicationPaymentRequestMessageListener {
    ApplicationPaymentRequestMessageListener::new(Arc::new(ProcessPayment::new(Arc::new(
        repository,
    ))))
}

fn command() -> ProcessPaymentCommand {
    ProcessPaymentCommand::new(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Decimal::ONE)
}

#[test]
fn test_payment_requested_processes_payment() -> anyhow::Result<()> {
    let mut repository = MockPaymentRepository::new();
    repository.expect_find_by_order_id().returning(|_| Ok(None));
    repository
        .expect_save_with_outbox()
        .once()
        .returning(|payment, _| Ok(payment));

    listener(repository).payment_requested(command())?;

    Ok(())
}

#[test]
fn test_payment_requested_is_idempotent_per_order() -> anyhow::Result<()> {
    let mut repository = MockPaymentRepository::new();
    repository.expect_find_by_order_id().returning(|order_id| {
        Ok(Some(Payment::new(
            PaymentId::new(),
            order_id,
            CustomerId::new(),
            Decimal::ONE,
            PaymentStatus::Completed,
        )))
    });
    repository.expect_save_with_outbox().never();

    listener(repository).payment_requested(command())?;

    Ok(())
}

#[test]
fn test_payment_requested_propagates_errors() {
    let mut repository = MockPaymentRepository::new();
    repository
        .expect_find_by_order_id()
        .returning(|_| Err(anyhow::anyhow!("database is down")));

    let result = listener(repository).payment_requested(command());

    assert_eq!("database is down", result.unwrap_err().to_string());
}

#[test]
fn test_order_id_is_used_for_idempotency() -> anyhow::Result<()> {
    let command = command();
    let order_id = OrderId::from_uuid(command.order_id());

    let mut repository = MockPaymentRepository::new();
    repository
        .expect_find_by_order_id()
        .withf(move |id| *id == order_id)
        .once()
        .returning(|_| Ok(None));
    repository
        .expect_save_with_outbox()
        .returning(|payment, _| Ok(payment));

    listener(repository).payment_requested(command)?;

    Ok(())
}
