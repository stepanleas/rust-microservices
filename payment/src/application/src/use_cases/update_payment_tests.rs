use crate::commands::UpdatePaymentCommand;
use crate::ports::out::publishers::MockPaymentMessagePublisher;
use crate::ports::out::repositories::MockPaymentRepository;
use crate::use_cases::UpdatePayment;
use domain::aggregates::payment::{Payment, PaymentId};
use domain::enums::PaymentStatus;
use mockall::predicate;
use rust_decimal::Decimal;
use shared::domain::value_objects::{CustomerId, OrderId};
use shared::error::AppError;
use std::sync::Arc;
use uuid::Uuid;

fn existing_payment(payment_id: Uuid, order_id: Uuid) -> Payment {
    Payment::new(
        PaymentId::from_uuid(payment_id),
        OrderId::from_uuid(order_id),
        CustomerId::new(),
        Decimal::new(10050, 2),
        PaymentStatus::Completed,
    )
}

#[test]
fn test_update_payment_execute() -> anyhow::Result<()> {
    let mut mock_repository = MockPaymentRepository::new();
    let mut mock_message_publisher = MockPaymentMessagePublisher::new();

    let payment_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let price = Decimal::new(10050, 2);

    mock_repository
        .expect_find_by_id()
        .once()
        .with(predicate::eq(PaymentId::from_uuid(payment_id)))
        .returning(move |_| Ok(Some(existing_payment(payment_id, order_id))));

    mock_repository
        .expect_save()
        .once()
        .withf(move |payment| {
            payment.id() == PaymentId::from_uuid(payment_id)
                && payment.price() == price
                && payment.payment_status() == PaymentStatus::Cancelled
        })
        .returning(Ok);

    mock_message_publisher
        .expect_publish_updated()
        .once()
        .withf(move |event| {
            event.payment().id() == PaymentId::from_uuid(payment_id)
                && event.payment().price() == price
        })
        .returning(|_| Ok(()));

    let use_case = UpdatePayment::new(Arc::new(mock_repository), Arc::new(mock_message_publisher));

    let result = use_case.execute(UpdatePaymentCommand::new(
        payment_id,
        order_id,
        customer_id,
        price,
        PaymentStatus::Cancelled,
    ))?;

    assert_eq!(result.price(), price);
    assert_eq!(result.payment_status(), PaymentStatus::Cancelled);

    Ok(())
}

#[test]
fn test_update_payment_execute_with_missing_payment() {
    let mut mock_repository = MockPaymentRepository::new();
    let mut mock_message_publisher = MockPaymentMessagePublisher::new();

    let payment_id = Uuid::new_v4();

    mock_repository
        .expect_find_by_id()
        .once()
        .returning(|_| Ok(None));
    mock_repository.expect_save().never();
    mock_message_publisher.expect_publish_updated().never();

    let use_case = UpdatePayment::new(Arc::new(mock_repository), Arc::new(mock_message_publisher));

    let error = use_case
        .execute(UpdatePaymentCommand::new(
            payment_id,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Decimal::ONE,
            PaymentStatus::Cancelled,
        ))
        .unwrap_err();

    assert!(matches!(error, AppError::NotFound(_)));
    assert_eq!(
        format!("Could not find payment with id: {payment_id}"),
        error.to_string()
    );
}
