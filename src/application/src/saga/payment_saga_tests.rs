use crate::outbox::{OrderApprovalCancelEventPayload, OutboxMessageType};
use crate::ports::out::repositories::MockOrderRepository;
use crate::ports::out::saga_store::MockOrderSagaRepository;
use crate::saga::dtos::{PaymentResponse, PaymentResponseStatus};
use crate::saga::payment_saga::PaymentSaga;
use domain::aggregates::order::{Order, OrderItem};
use domain::enums::OrderStatus;
use domain::saga::{OrderSaga, SagaStatus};
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CustomerId, OrderId, OrderItemId, ProductId};
use shared::error::AppError;
use std::sync::Arc;
use uuid::Uuid;

fn approved_order(order_id: OrderId, product_id: ProductId) -> anyhow::Result<Order> {
    Ok(Order::builder()
        .id(order_id)
        .customer_id(CustomerId::new())
        .price(Money::from_str("150.0", iso::USD)?)
        .items(vec![OrderItem::new(
            OrderItemId::new(),
            order_id,
            product_id,
            5,
            Money::from_str("30.0", iso::USD)?,
            Money::from_str("150.0", iso::USD)?,
        )])
        .status(OrderStatus::Approved)
        .build())
}

fn response(saga_id: Uuid, order_id: OrderId, status: PaymentResponseStatus) -> PaymentResponse {
    PaymentResponse {
        saga_id,
        order_id: *order_id.as_uuid(),
        payment_id: Uuid::new_v4(),
        status,
        failure_messages: match status {
            PaymentResponseStatus::Completed => vec![],
            _ => vec!["Payment declined".into()],
        },
    }
}

fn saga_in(
    status: SagaStatus,
    saga_id: Uuid,
    order_id: OrderId,
    product_id: ProductId,
) -> (MockOrderSagaRepository, MockOrderRepository) {
    let mut mock_saga_repository = MockOrderSagaRepository::new();
    let mut mock_order_repository = MockOrderRepository::new();

    mock_saga_repository
        .expect_find_by_id()
        .returning(move |_| Ok(Some(OrderSaga::restore(saga_id, order_id, status))));
    mock_order_repository
        .expect_find_by_id()
        .returning(move |_| Ok(Some(approved_order(order_id, product_id)?)));

    (mock_saga_repository, mock_order_repository)
}

fn payment_saga(
    saga_repository: MockOrderSagaRepository,
    order_repository: MockOrderRepository,
) -> PaymentSaga {
    PaymentSaga::new(Arc::new(saga_repository), Arc::new(order_repository))
}

#[test]
fn test_payment_completed_pays_order_and_finishes_saga() -> anyhow::Result<()> {
    let order_id = OrderId::new();
    let saga_id = Uuid::new_v4();
    let (mut mock_saga_repository, mock_order_repository) =
        saga_in(SagaStatus::Processing, saga_id, order_id, ProductId::new());

    mock_saga_repository
        .expect_advance()
        .once()
        .withf(|order, saga, expected_status, message| {
            order.status() == OrderStatus::Paid
                && saga.status() == SagaStatus::Succeeded
                && *expected_status == SagaStatus::Processing
                && message.is_none()
        })
        .returning(|_, _, _, _| Ok(true));

    payment_saga(mock_saga_repository, mock_order_repository).process(&response(
        saga_id,
        order_id,
        PaymentResponseStatus::Completed,
    ))?;

    Ok(())
}

#[test]
fn test_payment_failed_cancels_order_fails_saga_and_compensates_restaurant() -> anyhow::Result<()> {
    let order_id = OrderId::new();
    let product_id = ProductId::new();
    let saga_id = Uuid::new_v4();
    let (mut mock_saga_repository, mock_order_repository) =
        saga_in(SagaStatus::Processing, saga_id, order_id, product_id);

    mock_saga_repository
        .expect_advance()
        .once()
        .withf(move |order, saga, expected_status, message| {
            let Some(message) = message else {
                return false;
            };
            let Ok(payload) = message.payload_as::<OrderApprovalCancelEventPayload>() else {
                return false;
            };

            order.status() == OrderStatus::Canceled
                && order.failure_messages() == ["Payment declined".to_string()]
                && saga.status() == SagaStatus::Failed
                && *expected_status == SagaStatus::Processing
                && message.message_type() == OutboxMessageType::RestaurantApprovalCancelRequest
                && message.saga_id() == saga_id
                && payload.saga_id == saga_id
                && payload.order_id == *order_id.as_uuid()
                && payload.products.len() == 1
                && payload.products[0].id == *product_id.as_uuid()
                && payload.products[0].quantity == 5
        })
        .returning(|_, _, _, _| Ok(true));

    payment_saga(mock_saga_repository, mock_order_repository).rollback(&response(
        saga_id,
        order_id,
        PaymentResponseStatus::Failed,
    ))?;

    Ok(())
}

#[test]
fn test_payment_failure_before_restaurant_approval_is_ignored_without_compensation()
-> anyhow::Result<()> {
    let order_id = OrderId::new();
    let saga_id = Uuid::new_v4();
    let (mut mock_saga_repository, mock_order_repository) =
        saga_in(SagaStatus::Started, saga_id, order_id, ProductId::new());

    mock_saga_repository.expect_advance().never();

    payment_saga(mock_saga_repository, mock_order_repository).rollback(&response(
        saga_id,
        order_id,
        PaymentResponseStatus::Failed,
    ))?;

    Ok(())
}

#[test]
fn test_duplicate_payment_failure_is_ignored() -> anyhow::Result<()> {
    let order_id = OrderId::new();
    let saga_id = Uuid::new_v4();
    let (mut mock_saga_repository, mock_order_repository) =
        saga_in(SagaStatus::Failed, saga_id, order_id, ProductId::new());

    mock_saga_repository.expect_advance().never();

    payment_saga(mock_saga_repository, mock_order_repository).rollback(&response(
        saga_id,
        order_id,
        PaymentResponseStatus::Failed,
    ))?;

    Ok(())
}

#[test]
fn test_payment_reply_before_restaurant_approval_is_ignored() -> anyhow::Result<()> {
    let order_id = OrderId::new();
    let saga_id = Uuid::new_v4();
    let (mut mock_saga_repository, mock_order_repository) =
        saga_in(SagaStatus::Started, saga_id, order_id, ProductId::new());

    mock_saga_repository.expect_advance().never();

    payment_saga(mock_saga_repository, mock_order_repository).process(&response(
        saga_id,
        order_id,
        PaymentResponseStatus::Completed,
    ))?;

    Ok(())
}

#[test]
fn test_saga_moved_on_concurrently_is_not_an_error() -> anyhow::Result<()> {
    let order_id = OrderId::new();
    let saga_id = Uuid::new_v4();
    let (mut mock_saga_repository, mock_order_repository) =
        saga_in(SagaStatus::Processing, saga_id, order_id, ProductId::new());

    mock_saga_repository
        .expect_advance()
        .once()
        .returning(|_, _, _, _| Ok(false));

    payment_saga(mock_saga_repository, mock_order_repository).rollback(&response(
        saga_id,
        order_id,
        PaymentResponseStatus::Failed,
    ))?;

    Ok(())
}

#[test]
fn test_persistence_error_is_propagated() {
    let order_id = OrderId::new();
    let saga_id = Uuid::new_v4();
    let (mut mock_saga_repository, mock_order_repository) =
        saga_in(SagaStatus::Processing, saga_id, order_id, ProductId::new());

    mock_saga_repository
        .expect_advance()
        .once()
        .returning(|_, _, _, _| Err(anyhow::anyhow!("database is down")));

    let result = payment_saga(mock_saga_repository, mock_order_repository).process(&response(
        saga_id,
        order_id,
        PaymentResponseStatus::Completed,
    ));

    let error = result.unwrap_err();
    assert!(matches!(error, AppError::Internal(_)));
    assert_eq!("database is down", error.to_string());
}
