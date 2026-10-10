use crate::ports::out::repositories::MockPaymentRepository;
use crate::queries::FindPaymentByIdQuery;
use crate::use_cases::FindPaymentById;
use domain::aggregates::payment::{Payment, PaymentId};
use domain::enums::PaymentStatus;
use mockall::predicate;
use rust_decimal::Decimal;
use shared::domain::value_objects::{CustomerId, OrderId};
use shared::error::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn test_find_payment_by_id() -> anyhow::Result<()> {
    let payment_id = Uuid::new_v4();
    let mut mock_repository = MockPaymentRepository::new();

    mock_repository
        .expect_find_by_id()
        .once()
        .with(predicate::eq(PaymentId::from_uuid(payment_id)))
        .returning(move |_| {
            Ok(Some(Payment::new(
                PaymentId::from_uuid(payment_id),
                OrderId::new(),
                CustomerId::new(),
                Decimal::new(500, 2),
                PaymentStatus::Completed,
            )))
        });

    let use_case = FindPaymentById::new(Arc::new(mock_repository));
    let payment = use_case.execute(FindPaymentByIdQuery::new(payment_id))?;

    assert_eq!(payment_id, payment.id());
    assert_eq!(Decimal::new(500, 2), payment.price());

    Ok(())
}

#[test]
fn test_find_payment_by_id_not_found() {
    let payment_id = Uuid::new_v4();
    let mut mock_repository = MockPaymentRepository::new();

    mock_repository
        .expect_find_by_id()
        .once()
        .returning(|_| Ok(None));

    let use_case = FindPaymentById::new(Arc::new(mock_repository));
    let error = use_case
        .execute(FindPaymentByIdQuery::new(payment_id))
        .unwrap_err();

    assert!(matches!(error, AppError::NotFound(_)));
    assert_eq!(
        format!("Could not find payment with id: {payment_id}"),
        error.to_string()
    );
}

#[test]
fn test_find_payment_by_id_with_repository_error() {
    let mut mock_repository = MockPaymentRepository::new();

    mock_repository
        .expect_find_by_id()
        .once()
        .returning(|_| Err(anyhow::anyhow!("database is down")));

    let use_case = FindPaymentById::new(Arc::new(mock_repository));
    let error = use_case
        .execute(FindPaymentByIdQuery::new(Uuid::new_v4()))
        .unwrap_err();

    assert!(matches!(error, AppError::Internal(_)));
}
