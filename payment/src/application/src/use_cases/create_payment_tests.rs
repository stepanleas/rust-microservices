use crate::commands::CreatePaymentCommand;
use crate::ports::out::publishers::MockPaymentMessagePublisher;
use crate::ports::out::repositories::MockPaymentRepository;
use crate::use_cases::CreatePayment;
use domain::aggregates::payment::{Payment, PaymentId};
use domain::enums::PaymentStatus;
use mockall::predicate;
use rust_decimal::Decimal;
use shared::domain::value_objects::{CustomerId, OrderId};
use shared::error::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn test_create_payment_execute() -> anyhow::Result<()> {
    let mut mock_repository = MockPaymentRepository::new();
    let mut mock_message_publisher = MockPaymentMessagePublisher::new();

    let price = Decimal::new(10050, 2);
    let order_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();

    mock_repository
        .expect_find_by_order_id()
        .once()
        .with(predicate::eq(OrderId::from_uuid(order_id)))
        .returning(|_| Ok(None));

    mock_repository
        .expect_save()
        .once()
        .withf(move |payment| {
            payment.price() == price && payment.payment_status() == PaymentStatus::Completed
        })
        .returning(Ok);

    mock_message_publisher
        .expect_publish_created()
        .once()
        .withf(move |event| event.payment().price() == price)
        .returning(|_| Ok(()));

    let use_case = CreatePayment::new(Arc::new(mock_repository), Arc::new(mock_message_publisher));

    let result = use_case.execute(CreatePaymentCommand::new(order_id, customer_id, price))?;

    assert_eq!(result.price(), price);
    assert_eq!(result.payment_status(), PaymentStatus::Completed);

    Ok(())
}

#[test]
fn test_create_payment_execute_with_existing_order_payment() {
    let mut mock_repository = MockPaymentRepository::new();
    let mut mock_message_publisher = MockPaymentMessagePublisher::new();

    let order_id = Uuid::new_v4();

    mock_repository
        .expect_find_by_order_id()
        .once()
        .returning(move |_| {
            Ok(Some(Payment::new(
                PaymentId::new(),
                OrderId::from_uuid(order_id),
                CustomerId::new(),
                Decimal::new(10050, 2),
                PaymentStatus::Completed,
            )))
        });
    mock_repository.expect_save().never();
    mock_message_publisher.expect_publish_created().never();

    let use_case = CreatePayment::new(Arc::new(mock_repository), Arc::new(mock_message_publisher));

    let error = use_case
        .execute(CreatePaymentCommand::new(
            order_id,
            Uuid::new_v4(),
            Decimal::new(10050, 2),
        ))
        .unwrap_err();

    assert!(matches!(error, AppError::Conflict(_)));
    assert_eq!(
        format!("Payment for order with id: {} already exists", order_id),
        error.to_string(),
    );
}

#[test]
fn test_create_payment_execute_with_publisher_error() {
    let mut mock_repository = MockPaymentRepository::new();
    let mut mock_message_publisher = MockPaymentMessagePublisher::new();

    mock_repository
        .expect_find_by_order_id()
        .returning(|_| Ok(None));
    mock_repository.expect_save().returning(Ok);
    mock_message_publisher
        .expect_publish_created()
        .returning(|_| Err(anyhow::anyhow!("broker down")));

    let use_case = CreatePayment::new(Arc::new(mock_repository), Arc::new(mock_message_publisher));

    let error = use_case
        .execute(CreatePaymentCommand::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Decimal::ONE,
        ))
        .unwrap_err();

    assert!(matches!(error, AppError::Internal(_)));
}
