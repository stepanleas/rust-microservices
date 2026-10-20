use crate::outbox::{OrderPaymentEventPayload, OutboxMessageType};
use crate::ports::out::repositories::MockOrderRepository;
use crate::ports::out::saga_store::MockOrderSagaRepository;
use crate::saga::dtos::{OrderApprovalStatus, RestaurantApprovalResponse};
use crate::saga::restaurant_approval_saga::RestaurantApprovalSaga;
use domain::aggregates::order::Order;
use domain::enums::OrderStatus;
use domain::saga::{OrderSaga, SagaStatus};
use mockall::predicate;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CustomerId, OrderId};
use std::sync::Arc;
use uuid::Uuid;

fn pending_order(order_id: OrderId) -> anyhow::Result<Order> {
    Ok(Order::builder()
        .id(order_id)
        .customer_id(CustomerId::new())
        .price(Money::from_str("150.0", iso::USD)?)
        .status(OrderStatus::Pending)
        .build())
}

fn response(
    saga_id: Uuid,
    order_id: OrderId,
    status: OrderApprovalStatus,
) -> RestaurantApprovalResponse {
    RestaurantApprovalResponse {
        saga_id,
        order_id: *order_id.as_uuid(),
        status,
        failure_messages: match status {
            OrderApprovalStatus::Approved => vec![],
            OrderApprovalStatus::Rejected => vec!["Product is out of stock".into()],
        },
    }
}

#[test]
fn test_restaurant_approved_moves_order_to_approved_and_requests_payment() -> anyhow::Result<()> {
    let order_id = OrderId::new();
    let saga_id = Uuid::new_v4();

    let mut mock_saga_repository = MockOrderSagaRepository::new();
    let mut mock_order_repository = MockOrderRepository::new();

    mock_saga_repository
        .expect_find_by_id()
        .once()
        .with(predicate::eq(saga_id))
        .returning(move |_| {
            Ok(Some(OrderSaga::restore(
                saga_id,
                order_id,
                SagaStatus::Started,
                chrono::Utc::now(),
            )))
        });

    mock_order_repository
        .expect_find_by_id()
        .once()
        .with(predicate::eq(order_id))
        .returning(move |_| Ok(Some(pending_order(order_id)?)));

    mock_saga_repository
        .expect_advance()
        .once()
        .withf(move |order, saga, expected_status, message| {
            let Some(message) = message else {
                return false;
            };
            let Ok(payload) = message.payload_as::<OrderPaymentEventPayload>() else {
                return false;
            };

            order.status() == OrderStatus::Approved
                && saga.status() == SagaStatus::Processing
                && *expected_status == SagaStatus::Started
                && message.message_type() == OutboxMessageType::PaymentRequest
                && message.saga_id() == saga_id
                && payload.order_id == *order_id.as_uuid()
                && payload.price == "150.00"
        })
        .returning(|_, _, _, _| Ok(true));

    let saga = RestaurantApprovalSaga::new(
        Arc::new(mock_saga_repository),
        Arc::new(mock_order_repository),
    );

    saga.process(&response(saga_id, order_id, OrderApprovalStatus::Approved))?;

    Ok(())
}

#[test]
fn test_restaurant_rejected_cancels_order_and_fails_saga_without_compensation() -> anyhow::Result<()>
{
    let order_id = OrderId::new();
    let saga_id = Uuid::new_v4();

    let mut mock_saga_repository = MockOrderSagaRepository::new();
    let mut mock_order_repository = MockOrderRepository::new();

    mock_saga_repository
        .expect_find_by_id()
        .once()
        .returning(move |_| {
            Ok(Some(OrderSaga::restore(
                saga_id,
                order_id,
                SagaStatus::Started,
                chrono::Utc::now(),
            )))
        });

    mock_order_repository
        .expect_find_by_id()
        .once()
        .returning(move |_| Ok(Some(pending_order(order_id)?)));

    mock_saga_repository
        .expect_advance()
        .once()
        .withf(|order, saga, expected_status, message| {
            order.status() == OrderStatus::Canceled
                && order.failure_messages() == ["Product is out of stock".to_string()]
                && saga.status() == SagaStatus::Failed
                && *expected_status == SagaStatus::Started
                && message.is_none()
        })
        .returning(|_, _, _, _| Ok(true));

    let saga = RestaurantApprovalSaga::new(
        Arc::new(mock_saga_repository),
        Arc::new(mock_order_repository),
    );

    saga.rollback(&response(saga_id, order_id, OrderApprovalStatus::Rejected))?;

    Ok(())
}

#[test]
fn test_duplicate_restaurant_reply_is_ignored() -> anyhow::Result<()> {
    let order_id = OrderId::new();
    let saga_id = Uuid::new_v4();

    let mut mock_saga_repository = MockOrderSagaRepository::new();
    let mut mock_order_repository = MockOrderRepository::new();

    mock_saga_repository
        .expect_find_by_id()
        .times(2)
        .returning(move |_| {
            Ok(Some(OrderSaga::restore(
                saga_id,
                order_id,
                SagaStatus::Processing,
                chrono::Utc::now(),
            )))
        });
    mock_order_repository.expect_find_by_id().never();
    mock_saga_repository.expect_advance().never();

    let saga = RestaurantApprovalSaga::new(
        Arc::new(mock_saga_repository),
        Arc::new(mock_order_repository),
    );

    saga.process(&response(saga_id, order_id, OrderApprovalStatus::Approved))?;
    saga.rollback(&response(saga_id, order_id, OrderApprovalStatus::Rejected))?;

    Ok(())
}

#[test]
fn test_unknown_saga_is_ignored() -> anyhow::Result<()> {
    let mut mock_saga_repository = MockOrderSagaRepository::new();
    let mut mock_order_repository = MockOrderRepository::new();

    mock_saga_repository
        .expect_find_by_id()
        .once()
        .returning(|_| Ok(None));
    mock_order_repository.expect_find_by_id().never();
    mock_saga_repository.expect_advance().never();

    let saga = RestaurantApprovalSaga::new(
        Arc::new(mock_saga_repository),
        Arc::new(mock_order_repository),
    );

    saga.process(&response(
        Uuid::new_v4(),
        OrderId::new(),
        OrderApprovalStatus::Approved,
    ))?;

    Ok(())
}

#[test]
fn test_reply_for_another_order_is_an_error() {
    let saga_id = Uuid::new_v4();
    let saga_order_id = OrderId::new();

    let mut mock_saga_repository = MockOrderSagaRepository::new();
    let mut mock_order_repository = MockOrderRepository::new();

    mock_saga_repository
        .expect_find_by_id()
        .once()
        .returning(move |_| {
            Ok(Some(OrderSaga::restore(
                saga_id,
                saga_order_id,
                SagaStatus::Started,
                chrono::Utc::now(),
            )))
        });
    mock_order_repository.expect_find_by_id().never();
    mock_saga_repository.expect_advance().never();

    let saga = RestaurantApprovalSaga::new(
        Arc::new(mock_saga_repository),
        Arc::new(mock_order_repository),
    );

    let result = saga.process(&response(
        saga_id,
        OrderId::new(),
        OrderApprovalStatus::Approved,
    ));

    assert!(result.is_err());
}
