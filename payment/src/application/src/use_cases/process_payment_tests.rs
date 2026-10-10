use crate::commands::ProcessPaymentCommand;
use crate::outbox::{OutboxMessageType, PaymentResponsePayload};
use crate::ports::out::repositories::MockPaymentRepository;
use crate::use_cases::ProcessPayment;
use domain::aggregates::payment::{Payment, PaymentId};
use domain::enums::PaymentStatus;
use mockall::predicate;
use rust_decimal::Decimal;
use shared::domain::value_objects::{CustomerId, OrderId};
use shared::error::AppError;
use shared::outbox::OutboxStatus;
use std::sync::Arc;
use uuid::Uuid;

fn existing_payment(order_id: Uuid) -> Payment {
    Payment::new(
        PaymentId::new(),
        OrderId::from_uuid(order_id),
        CustomerId::new(),
        Decimal::new(10050, 2),
        PaymentStatus::Completed,
    )
}

#[test]
fn test_process_payment_completes_payment() -> anyhow::Result<()> {
    let mut mock_repository = MockPaymentRepository::new();

    let saga_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let price = Decimal::new(15000, 2);

    mock_repository
        .expect_find_by_order_id()
        .once()
        .with(predicate::eq(OrderId::from_uuid(order_id)))
        .returning(|_| Ok(None));

    mock_repository
        .expect_save_with_outbox()
        .once()
        .withf(move |payment, message| {
            let payload: PaymentResponsePayload = message.payload_as().unwrap();

            payment.order_id() == OrderId::from_uuid(order_id)
                && payment.customer_id() == CustomerId::from_uuid(customer_id)
                && payment.price() == price
                && payment.payment_status() == PaymentStatus::Completed
                && message.saga_id() == saga_id
                && message.message_type() == OutboxMessageType::PaymentResponse
                && message.status() == OutboxStatus::Started
                && payload.saga_id == saga_id
                && payload.payment_id == *payment.id().as_uuid()
                && payload.order_id == order_id
                && payload.price == "150.00"
                && payload.payment_status == "COMPLETED"
                && payload.failure_messages.is_empty()
        })
        .returning(|payment, _| Ok(payment));

    let use_case = ProcessPayment::new(Arc::new(mock_repository));
    let result = use_case
        .execute(ProcessPaymentCommand::new(
            saga_id,
            order_id,
            customer_id,
            price,
        ))?
        .expect("payment should be created");

    assert_eq!(order_id, result.order_id());
    assert_eq!(PaymentStatus::Completed, result.payment_status());

    Ok(())
}

#[test]
fn test_process_payment_fails_payment_for_non_positive_price() -> anyhow::Result<()> {
    let mut mock_repository = MockPaymentRepository::new();

    mock_repository
        .expect_find_by_order_id()
        .once()
        .returning(|_| Ok(None));

    mock_repository
        .expect_save_with_outbox()
        .once()
        .withf(|payment, message| {
            let payload: PaymentResponsePayload = message.payload_as().unwrap();

            payment.payment_status() == PaymentStatus::Failed
                && payload.payment_status == "FAILED"
                && payload.failure_messages == vec!["Total price must be greater than zero!"]
        })
        .returning(|payment, _| Ok(payment));

    let use_case = ProcessPayment::new(Arc::new(mock_repository));
    let result = use_case
        .execute(ProcessPaymentCommand::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Decimal::ZERO,
        ))?
        .expect("payment should be created");

    assert_eq!(PaymentStatus::Failed, result.payment_status());
    assert_eq!(
        vec!["Total price must be greater than zero!".to_string()],
        result.failure_messages(),
    );

    Ok(())
}

#[test]
fn test_process_payment_skips_duplicate_request() -> anyhow::Result<()> {
    let mut mock_repository = MockPaymentRepository::new();

    let order_id = Uuid::new_v4();

    mock_repository
        .expect_find_by_order_id()
        .once()
        .returning(move |_| Ok(Some(existing_payment(order_id))));
    mock_repository.expect_save_with_outbox().never();

    let use_case = ProcessPayment::new(Arc::new(mock_repository));
    let result = use_case.execute(ProcessPaymentCommand::new(
        Uuid::new_v4(),
        order_id,
        Uuid::new_v4(),
        Decimal::ONE,
    ))?;

    assert!(result.is_none());

    Ok(())
}

#[test]
fn test_process_payment_propagates_repository_error() {
    let mut mock_repository = MockPaymentRepository::new();

    mock_repository
        .expect_find_by_order_id()
        .once()
        .returning(|_| Ok(None));
    mock_repository
        .expect_save_with_outbox()
        .once()
        .returning(|_, _| Err(anyhow::anyhow!("database is down")));

    let use_case = ProcessPayment::new(Arc::new(mock_repository));
    let error = use_case
        .execute(ProcessPaymentCommand::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Decimal::ONE,
        ))
        .unwrap_err();

    assert!(matches!(error, AppError::Internal(_)));
    assert_eq!("database is down", error.to_string());
}
