use crate::saga::{OrderSaga, SagaStatus};
use shared::domain::value_objects::OrderId;
use uuid::Uuid;

#[test]
fn test_start_saga() {
    let order_id = OrderId::new();

    let saga = OrderSaga::start(order_id);

    assert_ne!(Uuid::nil(), saga.id());
    assert_eq!(order_id, saga.order_id());
    assert_eq!(SagaStatus::Started, saga.status());
}

#[test]
fn test_happy_path_transitions() -> anyhow::Result<()> {
    let mut saga = OrderSaga::start(OrderId::new());

    saga.restaurant_approved()?;
    assert_eq!(SagaStatus::Processing, saga.status());

    saga.payment_completed()?;
    assert_eq!(SagaStatus::Succeeded, saga.status());

    Ok(())
}

#[test]
fn test_fail_from_started_and_processing() -> anyhow::Result<()> {
    let mut rejected = OrderSaga::start(OrderId::new());
    rejected.fail()?;
    assert_eq!(SagaStatus::Failed, rejected.status());

    let mut payment_failed = OrderSaga::start(OrderId::new());
    payment_failed.restaurant_approved()?;
    payment_failed.fail()?;
    assert_eq!(SagaStatus::Failed, payment_failed.status());

    Ok(())
}

#[test]
fn test_invalid_transitions_are_rejected() {
    let mut saga = OrderSaga::start(OrderId::new());
    assert!(saga.payment_completed().is_err());

    let mut finished = OrderSaga::restore(Uuid::new_v4(), OrderId::new(), SagaStatus::Succeeded);
    assert!(finished.fail().is_err());
    assert!(finished.restaurant_approved().is_err());

    let mut failed = OrderSaga::restore(Uuid::new_v4(), OrderId::new(), SagaStatus::Failed);
    let error = failed.restaurant_approved().unwrap_err().to_string();
    assert!(
        error.contains("cannot move from FAILED to PROCESSING"),
        "{error}"
    );
}

#[test]
fn test_saga_status_round_trip() -> anyhow::Result<()> {
    for status in [
        SagaStatus::Started,
        SagaStatus::Processing,
        SagaStatus::Succeeded,
        SagaStatus::Failed,
    ] {
        assert_eq!(status, SagaStatus::try_from(status.as_str())?);
    }

    assert!(SagaStatus::try_from("UNKNOWN").is_err());

    Ok(())
}
