mod common;

use application::ports::out::repositories::{OrderApprovalRepository, ProductRepository};
use domain::aggregates::product::Product;
use domain::enums::OrderApprovalStatus;
use infrastructure::DbPool;
use infrastructure::postgres::{
    PostgresOrderApprovalRepository, PostgresOutboxRepository, PostgresProductRepository,
};
use messaging::publishers::PUBLISHED_TOPICS;
use rust_decimal::dec;
use shared::contracts::restaurant::RestaurantApprovalResponseAvroModel;
use shared::contracts::topics;
use shared::domain::value_objects::{OrderId, ProductId};
use shared::kafka::{KafkaProducer, MessageHandler, RetryPolicy};
use shared::outbox::OutboxStatus;
use shared::outbox::ports::OutboxRepository;
use starter::config::Settings;
use starter::wiring::{build_message_handler, build_outbox_relay};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

fn seed_product(pool: &DbPool, quantity: i32) -> anyhow::Result<ProductId> {
    let product_id = ProductId::new();
    PostgresProductRepository::new(pool).save(&Product::new(
        product_id,
        "Pizza".into(),
        quantity,
        dec!(10.00),
    ))?;

    Ok(product_id)
}

fn quantity(pool: &DbPool, product_id: ProductId) -> anyhow::Result<i32> {
    Ok(PostgresProductRepository::new(pool)
        .find_by_id(product_id)?
        .expect("product exists")
        .quantity())
}

fn approval_status(pool: &DbPool, order_id: Uuid) -> anyhow::Result<Option<OrderApprovalStatus>> {
    Ok(PostgresOrderApprovalRepository::new(pool)
        .find_by_order_id(OrderId::from_uuid(order_id))?
        .map(|approval| approval.status()))
}

fn relay_producer(host: &str) -> KafkaProducer {
    KafkaProducer::new(vec![host.to_string()], PUBLISHED_TOPICS)
}

/// Starts the real consumer loop in the background.
fn spawn_listener(pool: &DbPool, host: &str, group: &str) {
    let handler = Arc::new(build_message_handler(pool));
    let hosts = vec![host.to_string()];
    let group = group.to_string();
    std::thread::spawn(move || {
        shared::kafka::listen(hosts, group, handler, RetryPolicy::default())
    });
}

async fn wait_for(
    what: &str,
    mut condition: impl FnMut() -> anyhow::Result<bool>,
) -> anyhow::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !condition()? {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(())
}

#[tokio::test]
async fn test_approval_request_is_answered_on_kafka_via_outbox() -> anyhow::Result<()> {
    let (_postgres, pool) = common::start_postgres().await?;
    let (_kafka, host) = common::start_kafka().await?;
    common::ensure_topics(&host, &[topics::RESTAURANT_APPROVAL_RESPONSE])?;

    let product_id = seed_product(&pool, 5)?;
    let saga_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let request = common::approval_request(
        &saga_id.to_string(),
        &order_id.to_string(),
        &[(&product_id.as_uuid().to_string(), 2)],
    )?;

    // 1. The approval request is handled exactly like the Kafka listener would.
    let handler = build_message_handler(&pool);
    handler.handle(topics::RESTAURANT_APPROVAL_REQUEST, &request)?;

    assert_eq!(
        Some(OrderApprovalStatus::Approved),
        approval_status(&pool, order_id)?
    );
    assert_eq!(3, quantity(&pool, product_id)?);

    // Redelivery of the same request is ignored (idempotent consumer).
    handler.handle(topics::RESTAURANT_APPROVAL_REQUEST, &request)?;
    assert_eq!(3, quantity(&pool, product_id)?);
    let outbox = PostgresOutboxRepository::new(&pool);
    assert_eq!(1, outbox.find_by_status(OutboxStatus::Started, 10)?.len());

    // 2. The relay publishes the outbox message to Kafka.
    let relay = build_outbox_relay(&Settings::new(), &pool, relay_producer(&host));
    assert_eq!(1, relay.process_batch()?);
    assert!(outbox.find_by_status(OutboxStatus::Started, 10)?.is_empty());
    assert_eq!(1, outbox.find_by_status(OutboxStatus::Completed, 10)?.len());

    // 3. The response on Kafka follows the Avro contract.
    let response: RestaurantApprovalResponseAvroModel = common::decode_one(&common::consume_one(
        &host,
        topics::RESTAURANT_APPROVAL_RESPONSE,
    )?)?;
    assert_eq!(saga_id.to_string(), response.saga_id);
    assert_eq!(order_id.to_string(), response.order_id);
    assert_eq!("APPROVED", response.order_approval_status);
    assert!(response.failure_messages.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_rejected_order_is_consumed_from_kafka_and_answered() -> anyhow::Result<()> {
    let (_postgres, pool) = common::start_postgres().await?;
    let (_kafka, host) = common::start_kafka().await?;
    let handler = build_message_handler(&pool);
    common::ensure_topics(&host, &handler.topics())?;
    common::ensure_topics(&host, &[topics::RESTAURANT_APPROVAL_RESPONSE])?;

    let product_id = seed_product(&pool, 1)?;
    let saga_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    spawn_listener(&pool, &host, "restaurant-service-group-test");

    KafkaProducer::new(vec![host.clone()], &[topics::RESTAURANT_APPROVAL_REQUEST]).send(
        topics::RESTAURANT_APPROVAL_REQUEST,
        &order_id.to_string(),
        &common::approval_request(
            &saga_id.to_string(),
            &order_id.to_string(),
            &[(&product_id.as_uuid().to_string(), 4)],
        )?,
    )?;

    wait_for("approval request", || {
        Ok(approval_status(&pool, order_id)?.is_some())
    })
    .await?;

    assert_eq!(
        Some(OrderApprovalStatus::Rejected),
        approval_status(&pool, order_id)?
    );
    assert_eq!(
        1,
        quantity(&pool, product_id)?,
        "stock must not change when the order is rejected",
    );

    let relay = build_outbox_relay(&Settings::new(), &pool, relay_producer(&host));
    assert_eq!(1, relay.process_batch()?);

    let response: RestaurantApprovalResponseAvroModel = common::decode_one(&common::consume_one(
        &host,
        topics::RESTAURANT_APPROVAL_RESPONSE,
    )?)?;
    assert_eq!("REJECTED", response.order_approval_status);
    assert_eq!(
        vec!["Insufficient stock for product Pizza: requested 4, available 1".to_string()],
        response.failure_messages,
    );

    Ok(())
}

#[tokio::test]
async fn test_cancel_request_from_kafka_restocks_an_approved_order() -> anyhow::Result<()> {
    let (_postgres, pool) = common::start_postgres().await?;
    let (_kafka, host) = common::start_kafka().await?;
    let handler = build_message_handler(&pool);
    common::ensure_topics(&host, &handler.topics())?;

    let product_id = seed_product(&pool, 5)?;
    let saga_id = Uuid::new_v4().to_string();
    let order_id = Uuid::new_v4();
    let items = [(product_id.as_uuid().to_string(), 2)];
    let items: Vec<(&str, i32)> = items.iter().map(|(id, q)| (id.as_str(), *q)).collect();

    spawn_listener(&pool, &host, "restaurant-service-group-cancel-test");
    let mut producer = KafkaProducer::new(
        vec![host.clone()],
        &[
            topics::RESTAURANT_APPROVAL_REQUEST,
            topics::RESTAURANT_APPROVAL_CANCEL_REQUEST,
        ],
    );

    // 1. The order is approved and its stock taken.
    producer.send(
        topics::RESTAURANT_APPROVAL_REQUEST,
        &order_id.to_string(),
        &common::approval_request(&saga_id, &order_id.to_string(), &items)?,
    )?;
    wait_for("approval", || {
        Ok(approval_status(&pool, order_id)? == Some(OrderApprovalStatus::Approved))
    })
    .await?;
    assert_eq!(3, quantity(&pool, product_id)?);

    // 2. The payment failed: the order service asks to give the stock back (twice, as a
    //    redelivery would).
    let cancel = common::approval_cancel_request(&saga_id, &order_id.to_string(), &items)?;
    for _ in 0..2 {
        producer.send(
            topics::RESTAURANT_APPROVAL_CANCEL_REQUEST,
            &order_id.to_string(),
            &cancel,
        )?;
    }
    wait_for("cancellation", || {
        Ok(approval_status(&pool, order_id)? == Some(OrderApprovalStatus::Cancelled))
    })
    .await?;

    // The duplicate must not restock twice (give it time to be consumed too).
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(5, quantity(&pool, product_id)?);

    // No response message is written for the compensation.
    let outbox = PostgresOutboxRepository::new(&pool);
    assert_eq!(1, outbox.find_by_status(OutboxStatus::Started, 10)?.len());

    Ok(())
}

#[tokio::test]
async fn test_relay_recovers_when_response_topic_appears_later() -> anyhow::Result<()> {
    let (_postgres, pool) = common::start_postgres().await?;
    let (_kafka, host) = common::start_kafka().await?;

    let product_id = seed_product(&pool, 5)?;
    let order_id = Uuid::new_v4();
    build_message_handler(&pool).handle(
        topics::RESTAURANT_APPROVAL_REQUEST,
        &common::approval_request(
            &Uuid::new_v4().to_string(),
            &order_id.to_string(),
            &[(&product_id.as_uuid().to_string(), 1)],
        )?,
    )?;

    // The relay's producer is created before any topic exists on the broker.
    let relay = build_outbox_relay(&Settings::new(), &pool, relay_producer(&host));

    // Ticks are retried until the auto-created topic has partitions.
    let outbox = PostgresOutboxRepository::new(&pool);
    let deadline = Instant::now() + Duration::from_secs(30);
    while relay.process_batch()? == 0 {
        assert!(Instant::now() < deadline, "relay never published");
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    let completed = outbox.find_by_status(OutboxStatus::Completed, 10)?;
    assert_eq!(1, completed.len());
    assert!(completed[0].attempts() < Settings::new().outbox_max_attempts);

    let response: RestaurantApprovalResponseAvroModel = common::decode_one(&common::consume_one(
        &host,
        topics::RESTAURANT_APPROVAL_RESPONSE,
    )?)?;
    assert_eq!(order_id.to_string(), response.order_id);

    Ok(())
}
