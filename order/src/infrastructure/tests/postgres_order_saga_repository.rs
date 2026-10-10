mod common;

use application::outbox::{
    OrderApprovalCancelEventPayload, OrderApprovalEventPayload,
    OrderOutboxMessage as OutboxMessage, OrderPaymentEventPayload, OutboxMessageType,
};
use application::ports::out::repositories::{CustomerRepository, OrderRepository};
use application::ports::out::saga_store::OrderSagaRepository;
use domain::aggregates::customer::Customer;
use domain::aggregates::order::{Order, OrderItem};
use domain::enums::OrderStatus;
use domain::saga::{OrderSaga, SagaStatus};
use infrastructure::postgres::PostgresCustomerRepository;
use infrastructure::postgres::PostgresOrderRepository;
use infrastructure::postgres::PostgresOrderSagaRepository;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CustomerId, OrderId, OrderItemId, ProductId};
use shared::outbox::OutboxStatus;
use shared::outbox::ports::OutboxRepository;
use shared::outbox::postgres::PostgresOutboxRepository;

struct TestContext {
    _container: common::PostgresContainer,
    customer_repository: PostgresCustomerRepository,
    order_repository: PostgresOrderRepository,
    saga_repository: PostgresOrderSagaRepository,
    outbox_repository: PostgresOutboxRepository<OutboxMessageType>,
}

async fn setup_context() -> anyhow::Result<TestContext> {
    let (container, db_pool) = common::start_postgres().await?;

    Ok(TestContext {
        _container: container,
        customer_repository: PostgresCustomerRepository::new(&db_pool),
        order_repository: PostgresOrderRepository::new(&db_pool),
        saga_repository: PostgresOrderSagaRepository::new(&db_pool),
        outbox_repository: PostgresOutboxRepository::new(&db_pool),
    })
}

fn create_order(ctx: &TestContext) -> anyhow::Result<Order> {
    let customer_id = CustomerId::new();
    ctx.customer_repository.save(Customer::new(
        customer_id,
        "Artellas".into(),
        "Mike".into(),
        "Dane".into(),
    ))?;

    let order_id = OrderId::new();

    Ok(Order::builder()
        .id(order_id)
        .customer_id(customer_id)
        .price(Money::from_str("60.0", iso::USD)?)
        .items(vec![OrderItem::new(
            OrderItemId::new(),
            order_id,
            ProductId::new(),
            2,
            Money::from_str("30.0", iso::USD)?,
            Money::from_str("60.0", iso::USD)?,
        )])
        .build())
}

/// Starts a saga for a new order, the way the create order handler does.
fn start_saga(ctx: &TestContext) -> anyhow::Result<(Order, OrderSaga, OutboxMessage)> {
    let order = create_order(ctx)?;
    let saga = OrderSaga::start(order.id());
    let message = OutboxMessage::new(
        saga.id(),
        OutboxMessageType::RestaurantApprovalRequest,
        &OrderApprovalEventPayload::from_order(&order, saga.id()),
    )?;

    ctx.saga_repository.start(&order, &saga, &message)?;

    Ok((order, saga, message))
}

#[tokio::test]
async fn test_start_saga_persists_order_saga_and_outbox_message() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let (order, saga, message) = start_saga(&ctx)?;

    let saved_order = ctx
        .order_repository
        .find_by_id(order.id())?
        .expect("should be stored");
    assert_eq!(OrderStatus::Pending, saved_order.status());
    assert_eq!(order.tracking_id(), saved_order.tracking_id());
    assert_eq!(1, saved_order.items().len());

    let saved_saga = ctx
        .saga_repository
        .find_by_id(saga.id())?
        .expect("saga should be stored");
    assert_eq!(order.id(), saved_saga.order_id());
    assert_eq!(SagaStatus::Started, saved_saga.status());

    let pending = ctx
        .outbox_repository
        .find_by_status(OutboxStatus::Started, 10)?;
    assert_eq!(1, pending.len());
    assert_eq!(message.id(), pending[0].id());
    assert_eq!(saga.id(), pending[0].saga_id());
    assert_eq!(
        OutboxMessageType::RestaurantApprovalRequest,
        pending[0].message_type(),
    );

    let payload: OrderApprovalEventPayload = pending[0].payload_as()?;
    assert_eq!(*order.id().as_uuid(), payload.order_id);
    assert_eq!("60.00", payload.price);
    assert_eq!(2, payload.products[0].quantity);

    Ok(())
}

#[tokio::test]
async fn test_start_saga_is_atomic() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let (order, _, message) = start_saga(&ctx)?;

    // Same order id again: the order insert fails, so neither saga nor outbox row may be written.
    let second_saga = OrderSaga::start(order.id());
    let duplicate_message = OutboxMessage::new(
        second_saga.id(),
        OutboxMessageType::RestaurantApprovalRequest,
        &serde_json::json!({}),
    )?;

    let result = ctx
        .saga_repository
        .start(&order, &second_saga, &duplicate_message);
    assert!(result.is_err());

    assert!(ctx.saga_repository.find_by_id(second_saga.id())?.is_none());
    let pending = ctx
        .outbox_repository
        .find_by_status(OutboxStatus::Started, 10)?;
    assert_eq!(
        vec![message.id()],
        pending.iter().map(|m| m.id()).collect::<Vec<_>>()
    );

    Ok(())
}

#[tokio::test]
async fn test_advance_saga_updates_order_and_adds_outbox_message() -> anyhow::Result<()> {
    let ctx = setup_context().await?;
    let (order, saga, _) = start_saga(&ctx)?;

    let mut order = ctx
        .order_repository
        .find_by_id(order.id())?
        .expect("should be stored");
    let mut saga = ctx.saga_repository.find_by_id(saga.id())?.unwrap();
    order.approve()?;
    saga.restaurant_approved()?;
    let payment_request = OutboxMessage::new(
        saga.id(),
        OutboxMessageType::PaymentRequest,
        &OrderPaymentEventPayload::from_order(&order, saga.id()),
    )?;

    assert!(ctx.saga_repository.advance(
        &order,
        &saga,
        SagaStatus::Started,
        Some(payment_request.clone()),
    )?);

    assert_eq!(
        OrderStatus::Approved,
        ctx.order_repository
            .find_by_id(order.id())?
            .expect("should be stored")
            .status(),
    );
    assert_eq!(
        SagaStatus::Processing,
        ctx.saga_repository.find_by_id(saga.id())?.unwrap().status(),
    );
    let pending = ctx
        .outbox_repository
        .find_by_status(OutboxStatus::Started, 10)?;
    assert!(
        pending
            .iter()
            .any(|message| message.id() == payment_request.id())
    );

    // Payment fails afterwards: order cancelled with the failure reason, saga failed.
    // The restaurant compensation is stored in the same transaction.
    order.cancel(vec!["Payment declined".into()])?;
    saga.fail()?;
    let cancel_request = OutboxMessage::new(
        saga.id(),
        OutboxMessageType::RestaurantApprovalCancelRequest,
        &OrderApprovalCancelEventPayload::from_order(&order, saga.id()),
    )?;
    assert!(ctx.saga_repository.advance(
        &order,
        &saga,
        SagaStatus::Processing,
        Some(cancel_request.clone()),
    )?);

    let cancelled = ctx
        .order_repository
        .find_by_tracking_id(order.tracking_id())?
        .expect("should be stored");
    assert_eq!(OrderStatus::Canceled, cancelled.status());
    assert_eq!(
        vec!["Payment declined".to_string()],
        cancelled.failure_messages()
    );
    let pending = ctx
        .outbox_repository
        .find_by_status(OutboxStatus::Started, 10)?;
    let stored_cancel_request = pending
        .iter()
        .find(|message| message.id() == cancel_request.id())
        .expect("cancel request should be stored");
    assert_eq!(
        OutboxMessageType::RestaurantApprovalCancelRequest,
        stored_cancel_request.message_type()
    );
    let payload: OrderApprovalCancelEventPayload = stored_cancel_request.payload_as()?;
    assert_eq!(*order.id().as_uuid(), payload.order_id);
    assert_eq!(1, payload.products.len());
    assert_eq!(2, payload.products[0].quantity);

    assert_eq!(
        SagaStatus::Failed,
        ctx.saga_repository.find_by_id(saga.id())?.unwrap().status(),
    );

    Ok(())
}

#[tokio::test]
async fn test_advance_with_stale_status_changes_nothing() -> anyhow::Result<()> {
    let ctx = setup_context().await?;
    let (order, saga, _) = start_saga(&ctx)?;

    let mut order = ctx
        .order_repository
        .find_by_id(order.id())?
        .expect("should be stored");
    let mut saga = ctx.saga_repository.find_by_id(saga.id())?.unwrap();
    order.approve()?;
    saga.restaurant_approved()?;
    let message = OutboxMessage::new(
        saga.id(),
        OutboxMessageType::PaymentRequest,
        &serde_json::json!({}),
    )?;

    // The saga is STARTED in the database, so expecting PROCESSING must change nothing.
    let advanced = ctx.saga_repository.advance(
        &order,
        &saga,
        SagaStatus::Processing,
        Some(message.clone()),
    )?;
    assert!(!advanced);

    assert_eq!(
        OrderStatus::Pending,
        ctx.order_repository
            .find_by_id(order.id())?
            .expect("should be stored")
            .status(),
    );
    assert_eq!(
        SagaStatus::Started,
        ctx.saga_repository.find_by_id(saga.id())?.unwrap().status(),
    );
    let pending = ctx
        .outbox_repository
        .find_by_status(OutboxStatus::Started, 10)?;
    assert!(pending.iter().all(|pending| pending.id() != message.id()));

    Ok(())
}

#[tokio::test]
async fn test_find_unknown_saga_returns_none() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    assert!(
        ctx.saga_repository
            .find_by_id(uuid::Uuid::new_v4())?
            .is_none()
    );

    Ok(())
}
