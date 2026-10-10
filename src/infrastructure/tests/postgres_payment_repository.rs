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
    repository: PostgresPaymentRepository,
    outbox_repository: PostgresOutboxRepository<OutboxMessageType>,
    pool: DbPool,
}

async fn setup_context() -> anyhow::Result<TestContext> {
    let (container, db_pool) = common::start_postgres().await?;

    Ok(TestContext {
        _container: container,
        repository: PostgresPaymentRepository::new(&db_pool),
        outbox_repository: PostgresOutboxRepository::new(&db_pool),
        pool: db_pool,
    })
}

fn payment(order_id: OrderId, price: Decimal) -> Payment {
    Payment::new(
        PaymentId::new(),
        order_id,
        CustomerId::new(),
        price,
        PaymentStatus::Completed,
    )
}

fn outbox_message(payload: serde_json::Value) -> anyhow::Result<PaymentOutboxMessage> {
    PaymentOutboxMessage::new(Uuid::new_v4(), OutboxMessageType::PaymentResponse, &payload)
}

#[tokio::test]
async fn test_create_and_find_payment() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let order_id = OrderId::new();
    let price = Decimal::new(10050, 2);
    let payment = payment(order_id, price);
    let payment_id = payment.id();

    ctx.repository.save(payment)?;

    let by_id = ctx
        .repository
        .find_by_id(payment_id)?
        .expect("payment should exist");
    assert_eq!(payment_id, by_id.id());
    assert_eq!(order_id, by_id.order_id());
    assert_eq!(price, by_id.price());
    assert_eq!(PaymentStatus::Completed, by_id.payment_status());
    assert!(by_id.failure_messages().is_empty());

    let by_order_id = ctx
        .repository
        .find_by_order_id(order_id)?
        .expect("payment should exist");
    assert_eq!(payment_id, by_order_id.id());

    assert!(ctx.repository.find_by_order_id(OrderId::new())?.is_none());

    Ok(())
}

#[tokio::test]
async fn test_update_payment() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let order_id = OrderId::new();
    let saved = ctx.repository.save(payment(order_id, Decimal::ONE))?;

    let updated = Payment::builder()
        .id(saved.id())
        .order_id(order_id)
        .customer_id(saved.customer_id())
        .price(Decimal::TEN)
        .payment_status(PaymentStatus::Cancelled)
        .build();
    ctx.repository.save(updated)?;

    let found = ctx
        .repository
        .find_by_id(saved.id())?
        .expect("payment should exist");
    assert_eq!(Decimal::TEN, found.price());
    assert_eq!(PaymentStatus::Cancelled, found.payment_status());

    Ok(())
}

#[tokio::test]
async fn test_find_missing_payment_returns_none() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    assert!(ctx.repository.find_by_id(PaymentId::new())?.is_none());

    Ok(())
}

#[tokio::test]
async fn test_save_with_outbox_persists_payment_and_message() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let saga_id = Uuid::new_v4();
    let order_id = OrderId::new();
    let mut failed_payment = payment(order_id, Decimal::ZERO);
    failed_payment.process();

    let message = PaymentOutboxMessage::new(
        saga_id,
        OutboxMessageType::PaymentResponse,
        &serde_json::json!({ "order_id": order_id.as_uuid(), "payment_status": "FAILED" }),
    )?;
    let message_id = message.id();

    ctx.repository.save_with_outbox(failed_payment, message)?;

    let saved_payment = ctx
        .repository
        .find_by_order_id(order_id)?
        .expect("payment should exist");
    assert_eq!(PaymentStatus::Failed, saved_payment.payment_status());
    assert_eq!(
        vec!["Total price must be greater than zero!".to_string()],
        saved_payment.failure_messages(),
    );

    let saved_message =
        common::find_outbox_message(&ctx.pool, message_id)?.expect("message should exist");
    assert_eq!(saga_id, saved_message.saga_id);
    assert_eq!("PAYMENT_RESPONSE", saved_message.message_type);
    assert_eq!("STARTED", saved_message.outbox_status);
    assert_eq!(0, saved_message.attempts);
    assert_eq!("FAILED", saved_message.payload["payment_status"]);

    Ok(())
}

#[tokio::test]
async fn test_save_with_outbox_rolls_back_on_duplicate_order() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let order_id = OrderId::new();
    ctx.repository.save_with_outbox(
        payment(order_id, Decimal::ONE),
        outbox_message(serde_json::json!({}))?,
    )?;

    let duplicate_message = outbox_message(serde_json::json!({}))?;
    let duplicate_message_id = duplicate_message.id();

    let result = ctx
        .repository
        .save_with_outbox(payment(order_id, Decimal::TEN), duplicate_message);

    assert!(result.is_err(), "order_id must be unique");
    assert!(
        common::find_outbox_message(&ctx.pool, duplicate_message_id)?.is_none(),
        "outbox message must be rolled back together with the payment"
    );
    assert_eq!(
        1,
        ctx.outbox_repository
            .find_by_status(OutboxStatus::Started, 10)?
            .len()
    );

    Ok(())
}

#[tokio::test]
async fn test_save_rejects_second_payment_for_same_order() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let order_id = OrderId::new();
    ctx.repository.save(payment(order_id, Decimal::ONE))?;

    assert!(
        ctx.repository
            .save(payment(order_id, Decimal::ONE))
            .is_err()
    );

    Ok(())
}
