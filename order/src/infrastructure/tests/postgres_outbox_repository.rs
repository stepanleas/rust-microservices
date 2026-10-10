mod common;

use application::outbox::{OrderOutboxMessage as OutboxMessage, OutboxMessageType};
use application::ports::out::repositories::CustomerRepository;
use application::ports::out::saga_store::OrderSagaRepository;
use domain::aggregates::customer::Customer;
use domain::aggregates::order::Order;
use domain::saga::OrderSaga;
use infrastructure::postgres::PostgresCustomerRepository;
use infrastructure::postgres::PostgresOrderSagaRepository;
use shared::domain::value_objects::{CustomerId, OrderId};
use shared::outbox::OutboxStatus;
use shared::outbox::ports::OutboxRepository;
use shared::outbox::postgres::PostgresOutboxRepository;

struct TestContext {
    _container: common::PostgresContainer,
    customer_repository: PostgresCustomerRepository,
    saga_repository: PostgresOrderSagaRepository,
    outbox_repository: PostgresOutboxRepository<OutboxMessageType>,
}

async fn setup_context() -> anyhow::Result<TestContext> {
    let (container, db_pool) = common::start_postgres().await?;

    Ok(TestContext {
        _container: container,
        customer_repository: PostgresCustomerRepository::new(&db_pool),
        saga_repository: PostgresOrderSagaRepository::new(&db_pool),
        outbox_repository: PostgresOutboxRepository::new(&db_pool),
    })
}

/// Stores an order with its saga and returns the outbox message written alongside it.
fn store_message(ctx: &TestContext) -> anyhow::Result<OutboxMessage> {
    let customer_id = CustomerId::new();
    ctx.customer_repository.save(Customer::new(
        customer_id,
        "Artellas".into(),
        "Mike".into(),
        "Dane".into(),
    ))?;

    let order = Order::builder()
        .id(OrderId::new())
        .customer_id(customer_id)
        .build();
    let saga = OrderSaga::start(order.id());
    let message = OutboxMessage::new(
        saga.id(),
        OutboxMessageType::RestaurantApprovalRequest,
        &serde_json::json!({ "order_id": order.id().as_uuid() }),
    )?;

    ctx.saga_repository.start(&order, &saga, &message)?;

    Ok(message)
}

#[tokio::test]
async fn test_find_by_status_returns_oldest_first_and_respects_limit() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let first = store_message(&ctx)?;
    let second = store_message(&ctx)?;
    store_message(&ctx)?;

    let pending = ctx
        .outbox_repository
        .find_by_status(OutboxStatus::Started, 2)?;

    assert_eq!(
        vec![first.id(), second.id()],
        pending
            .iter()
            .map(|message| message.id())
            .collect::<Vec<_>>(),
    );
    assert_eq!(first.payload(), pending[0].payload());
    assert_eq!(0, pending[0].attempts());

    Ok(())
}

#[tokio::test]
async fn test_mark_completed() -> anyhow::Result<()> {
    let ctx = setup_context().await?;
    let message = store_message(&ctx)?;

    ctx.outbox_repository.mark_completed(message.id())?;

    assert!(
        ctx.outbox_repository
            .find_by_status(OutboxStatus::Started, 10)?
            .is_empty()
    );
    let completed = ctx
        .outbox_repository
        .find_by_status(OutboxStatus::Completed, 10)?;
    assert_eq!(message.id(), completed[0].id());

    Ok(())
}

#[tokio::test]
async fn test_mark_attempt_failed_until_max_attempts() -> anyhow::Result<()> {
    let ctx = setup_context().await?;
    let message = store_message(&ctx)?;

    ctx.outbox_repository.mark_attempt_failed(message.id(), 2)?;

    let pending = ctx
        .outbox_repository
        .find_by_status(OutboxStatus::Started, 10)?;
    assert_eq!(1, pending[0].attempts());

    ctx.outbox_repository.mark_attempt_failed(message.id(), 2)?;

    assert!(
        ctx.outbox_repository
            .find_by_status(OutboxStatus::Started, 10)?
            .is_empty()
    );
    let failed = ctx
        .outbox_repository
        .find_by_status(OutboxStatus::Failed, 10)?;
    assert_eq!(message.id(), failed[0].id());
    assert_eq!(2, failed[0].attempts());

    Ok(())
}
