use crate::ports::out::repositories::MockPaymentRepository;
use crate::queries::FindPaymentByOrderIdQuery;
use crate::use_cases::FindPaymentByOrderId;
use domain::aggregates::payment::{Payment, PaymentId};
use domain::enums::PaymentStatus;
use mockall::predicate;
use rust_decimal::Decimal;
use shared::domain::value_objects::{CustomerId, OrderId};
use shared::error::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn test_find_payment_by_order_id() -> anyhow::Result<()> {
    let order_id = Uuid::new_v4();
    let mut mock_repository = MockPaymentRepository::new();

    mock_repository
        .expect_find_by_order_id()
        .once()
        .with(predicate::eq(OrderId::from_uuid(order_id)))
        .returning(move |_| {
            Ok(Some(Payment::new(
                PaymentId::new(),
                OrderId::from_uuid(order_id),
                CustomerId::new(),
                Decimal::new(500, 2),
                PaymentStatus::Completed,
            )))
        });

    let use_case = FindPaymentByOrderId::new(Arc::new(mock_repository));
    let payment = use_case.execute(FindPaymentByOrderIdQuery::new(order_id))?;

    assert_eq!(order_id, payment.order_id());

    Ok(())
}

#[test]
fn test_find_payment_by_order_id_not_found() {
    let order_id = Uuid::new_v4();
    let mut mock_repository = MockPaymentRepository::new();

    mock_repository
        .expect_find_by_order_id()
        .once()
        .returning(|_| Ok(None));

    let use_case = FindPaymentByOrderId::new(Arc::new(mock_repository));
    let error = use_case
        .execute(FindPaymentByOrderIdQuery::new(order_id))
        .unwrap_err();

    assert!(matches!(error, AppError::NotFound(_)));
    assert_eq!(
        format!("Could not find payment for order with id: {}", order_id),
        error.to_string(),
    );
}
