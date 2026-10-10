//! Runs the shared Postgres outbox repository against this service's migrations.

mod common;

use application::outbox::{OutboxMessageType, PaymentOutboxMessage};
use application::ports::out::repositories::PaymentRepository;
use domain::aggregates::payment::{Payment, PaymentId};
use domain::enums::PaymentStatus;
use infrastructure::DbPool;
use infrastructure::postgres::PostgresPaymentRepository;
use rust_decimal::Decimal;
use shared::domain::value_objects::{CustomerId, OrderId};
use shared::outbox::OutboxStatus;
use shared::outbox::ports::OutboxRepository;
use shared::outbox::postgres::PostgresOutboxRepository;
use uuid::Uuid;

struct TestContext {
    _container: common::PostgresContainer,
    pool: DbPool,
    payment_repository: PostgresPaymentRepository,
    outbox_repository: PostgresOutboxRepository<OutboxMessageType>,
}

async fn setup_context() -> anyhow::Result<TestContext> {
    let (container, db_pool) = common::start_postgres().await?;

    Ok(TestContext {
        _container: container,
        payment_repository: PostgresPaymentRepository::new(&db_pool),
        outbox_repository: PostgresOutboxRepository::new(&db_pool),
        pool: db_pool,
    })
}

fn insert_message(ctx: &TestContext) -> anyhow::Result<PaymentOutboxMessage> {
    let message = PaymentOutboxMessage::new(
        Uuid::new_v4(),
        OutboxMessageType::PaymentResponse,
        &serde_json::json!({ "value": 1 }),
    )?;

    ctx.payment_repository.save_with_outbox(
        Payment::new(
            PaymentId::new(),
            OrderId::new(),
            CustomerId::new(),
            Decimal::ONE,
            PaymentStatus::Completed,
        ),
        message.clone(),
    )?;

    Ok(message)
}

#[tokio::test]
async fn test_find_by_status_orders_by_creation_and_limits() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let first = insert_message(&ctx)?;
    std::thread::sleep(std::time::Duration::from_millis(5));
    let second = insert_message(&ctx)?;
    std::thread::sleep(std::time::Duration::from_millis(5));
    insert_message(&ctx)?;

    let started = ctx
        .outbox_repository
        .find_by_status(OutboxStatus::Started, 2)?;

    assert_eq!(2, started.len());
    assert_eq!(first.id(), started[0].id());
    assert_eq!(second.id(), started[1].id());
    assert_eq!(
        OutboxMessageType::PaymentResponse,
        started[0].message_type()
    );
    assert_eq!(&serde_json::json!({ "value": 1 }), started[0].payload());
    assert!(
        ctx.outbox_repository
            .find_by_status(OutboxStatus::Completed, 10)?
            .is_empty()
    );

    Ok(())
}

#[tokio::test]
async fn test_mark_completed() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let message = insert_message(&ctx)?;
    ctx.outbox_repository.mark_completed(message.id())?;

    let saved = common::find_outbox_message(&ctx.pool, message.id())?.expect("message exists");
    assert_eq!("COMPLETED", saved.outbox_status);
    assert!(saved.processed_at.is_some());
    assert!(
        ctx.outbox_repository
            .find_by_status(OutboxStatus::Started, 10)?
            .is_empty()
    );

    Ok(())
}

#[tokio::test]
async fn test_mark_attempt_failed_until_max_attempts() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let message = insert_message(&ctx)?;

    assert_eq!(
        OutboxStatus::Started,
        ctx.outbox_repository.mark_attempt_failed(message.id(), 3)?
    );
    assert_eq!(
        OutboxStatus::Started,
        ctx.outbox_repository.mark_attempt_failed(message.id(), 3)?
    );
    assert_eq!(
        OutboxStatus::Failed,
        ctx.outbox_repository.mark_attempt_failed(message.id(), 3)?
    );

    let saved = common::find_outbox_message(&ctx.pool, message.id())?.expect("message exists");
    assert_eq!(3, saved.attempts);
    assert_eq!("FAILED", saved.outbox_status);
    assert!(saved.processed_at.is_some());

    Ok(())
}
