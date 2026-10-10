//! Runs the order side of the saga against real Postgres and Kafka containers.
//!
//! The restaurant and payment services are simulated by producing their replies to Kafka; the
//! order service consumes them with its real Kafka listener.

use application::commands::{CreateOrderCommand, CreateOrderItemDto};
use application::outbox::OutboxMessageType;
use application::ports::out::repositories::{
    CustomerRepository, OrderRepository, ProductRepository,
};
use application::ports::out::saga_store::OrderSagaRepository;
use domain::aggregates::customer::Customer;
use domain::aggregates::order::Order;
use domain::aggregates::product::Product;
use domain::enums::OrderStatus;
use domain::saga::SagaStatus;
use infrastructure::postgres::{
    PostgresCustomerRepository, PostgresOrderRepository, PostgresOrderSagaRepository,
    PostgresProductRepository,
};
use rust_decimal::dec;
use rusty_money::{Money, iso};
use shared::contracts::AvroMessage;
use shared::contracts::payment::{PaymentRequestAvroModel, PaymentResponseAvroModel};
use shared::contracts::restaurant::{
    RestaurantApprovalCancelRequestAvroModel, RestaurantApprovalRequestAvroModel,
    RestaurantApprovalResponseAvroModel,
};
use shared::contracts::topics;
use shared::db::DbSettings;
use shared::domain::value_objects::{CustomerId, ProductId};
use shared::kafka::{KafkaConsumer, KafkaProducer};
use shared::outbox::OutboxStatus;
use shared::outbox::ports::OutboxRepository;
use shared::outbox::postgres::PostgresOutboxRepository;
use shared::outbox::relay::{OutboxRelay, RelaySettings};
use shared::shutdown::Shutdown;
use starter::wiring::{UseCases, outbox_relay, start_kafka_listener};
use std::sync::Arc;
use std::time::{Duration, Instant};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::kafka::apache::{KAFKA_PORT, Kafka};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

const TIMEOUT: Duration = Duration::from_secs(60);
/// Same major version as docker-compose and the Helm chart.
const KAFKA_VERSION: &str = "4.3.1";

struct TestContext {
    _postgres: ContainerAsync<Postgres>,
    _kafka: ContainerAsync<Kafka>,
    kafka_host: String,
    customer_repository: Arc<PostgresCustomerRepository>,
    product_repository: Arc<PostgresProductRepository>,
    order_repository: Arc<PostgresOrderRepository>,
    saga_repository: Arc<PostgresOrderSagaRepository>,
    outbox_repository: Arc<PostgresOutboxRepository<OutboxMessageType>>,
    use_cases: UseCases,
    relay: OutboxRelay<OutboxMessageType>,
}

async fn setup() -> anyhow::Result<TestContext> {
    let postgres = Postgres::default().start().await?;
    let kafka = Kafka::default().with_tag(KAFKA_VERSION).start().await?;

    let database_url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        postgres.get_host_port_ipv4(5432).await?,
    );
    let kafka_host = format!("127.0.0.1:{}", kafka.get_host_port_ipv4(KAFKA_PORT).await?);

    let settings = DbSettings {
        url: database_url,
        max_connections: 8,
        connection_timeout: Duration::from_secs(5),
    };
    let pool = tokio::task::spawn_blocking(move || {
        shared::db::connect(&settings, infrastructure::MIGRATIONS)
    })
    .await??;

    shared::kafka::ensure_topics(
        std::slice::from_ref(&kafka_host),
        &[
            topics::RESTAURANT_APPROVAL_REQUEST,
            topics::PAYMENT_REQUEST,
            topics::RESTAURANT_APPROVAL_CANCEL_REQUEST,
            topics::RESTAURANT_APPROVAL_RESPONSE,
            topics::PAYMENT_RESPONSE,
        ],
        3,
        TIMEOUT,
    )?;

    // The production wiring: use cases, Kafka listener and outbox relay.
    let use_cases = UseCases::new(&pool);
    start_kafka_listener(
        &kafka_host,
        &format!("order-test-{}", Uuid::new_v4()),
        use_cases.message_handler(),
        &Shutdown::new(),
    );
    let relay = outbox_relay(
        &pool,
        &kafka_host,
        RelaySettings {
            poll_interval: Duration::from_millis(100),
            batch_size: 100,
            max_attempts: 5,
            retention: Duration::from_secs(3600),
        },
    );

    Ok(TestContext {
        _postgres: postgres,
        _kafka: kafka,
        kafka_host,
        customer_repository: Arc::new(PostgresCustomerRepository::new(&pool)),
        product_repository: Arc::new(PostgresProductRepository::new(&pool)),
        order_repository: Arc::new(PostgresOrderRepository::new(&pool)),
        saga_repository: Arc::new(PostgresOrderSagaRepository::new(&pool)),
        outbox_repository: Arc::new(PostgresOutboxRepository::new(&pool)),
        use_cases,
        relay,
    })
}

/// Reads the first message of a topic with a fresh consumer group.
fn consume_one(kafka_host: &str, topic: &str) -> anyhow::Result<Vec<u8>> {
    let consumer = KafkaConsumer::new(
        vec![kafka_host.to_string()],
        format!("test-{}", Uuid::new_v4()),
        &[topic],
    )?;
    let started = Instant::now();

    while started.elapsed() < TIMEOUT {
        if let Some(message) = consumer.poll(Duration::from_millis(500))? {
            return Ok(message.payload);
        }
    }

    anyhow::bail!("no message on topic {topic}")
}

fn produce(kafka_host: &str, topic: &str, key: &str, payload: &[u8]) -> anyhow::Result<()> {
    KafkaProducer::new(vec![kafka_host.to_string()]).send(topic, key, payload)
}

fn wait_for_order_status(
    ctx: &TestContext,
    tracking_id: Uuid,
    expected: OrderStatus,
) -> anyhow::Result<Order> {
    let started = Instant::now();

    while started.elapsed() < TIMEOUT {
        if let Some(order) = ctx.order_repository.find_by_tracking_id(tracking_id)?
            && order.status() == expected
        {
            return Ok(order);
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    anyhow::bail!("order {tracking_id} did not reach {expected:?}")
}

fn place_order(ctx: &TestContext) -> anyhow::Result<Uuid> {
    let customer_id = CustomerId::new();
    let product_id = ProductId::new();

    ctx.customer_repository.save(Customer::new(
        customer_id,
        "Artellas".into(),
        "Mike".into(),
        "Dane".into(),
    ))?;
    ctx.product_repository.save(Product::new(
        product_id,
        "Pizza".into(),
        10,
        Money::from_str("30.0", iso::USD)?,
    ))?;

    Ok(ctx.use_cases.create_order.execute(CreateOrderCommand::new(
        *customer_id.as_uuid(),
        dec!(60.0),
        vec![CreateOrderItemDto::new(
            *product_id.as_uuid(),
            2,
            dec!(30.0),
            dec!(60.0),
        )],
    ))?)
}

/// Publishes the restaurant approval request and returns it as the restaurant would read it.
fn relay_approval_request(ctx: &TestContext) -> anyhow::Result<RestaurantApprovalRequestAvroModel> {
    assert_eq!(1, ctx.relay.process_batch()?);

    let payload = consume_one(&ctx.kafka_host, topics::RESTAURANT_APPROVAL_REQUEST)?;
    let mut requests = RestaurantApprovalRequestAvroModel::from_avro_bytes(&payload)?;

    Ok(requests.remove(0))
}

fn reply_from_restaurant(
    ctx: &TestContext,
    request: &RestaurantApprovalRequestAvroModel,
    status: &str,
    failure_messages: Vec<String>,
) -> anyhow::Result<()> {
    let reply = RestaurantApprovalResponseAvroModel {
        id: Uuid::new_v4().to_string(),
        saga_id: request.saga_id.clone(),
        order_id: request.order_id.clone(),
        order_approval_status: status.into(),
        failure_messages,
        created_at: "2026-10-07T10:00:00+00:00".into(),
    };

    produce(
        &ctx.kafka_host,
        topics::RESTAURANT_APPROVAL_RESPONSE,
        &request.order_id,
        &reply.to_avro_bytes()?,
    )
}

#[tokio::test]
async fn test_order_saga_happy_path_ends_paid() -> anyhow::Result<()> {
    let ctx = setup().await?;
    let tracking_id = place_order(&ctx)?;

    // Step 1: order PENDING, restaurant approval request relayed to Kafka.
    let order = ctx
        .order_repository
        .find_by_tracking_id(tracking_id)?
        .expect("order should be stored");
    assert_eq!(OrderStatus::Pending, order.status());

    let request = relay_approval_request(&ctx)?;
    assert_eq!(order.id().as_uuid().to_string(), request.order_id);
    assert_eq!("60.00", request.price);
    assert_eq!(1, request.products.len());
    assert_eq!(2, request.products[0].quantity);

    let saga_id = Uuid::parse_str(&request.saga_id)?;

    // Step 2: restaurant approves (delivered twice to prove idempotency).
    reply_from_restaurant(&ctx, &request, "APPROVED", vec![])?;
    reply_from_restaurant(&ctx, &request, "APPROVED", vec![])?;
    wait_for_order_status(&ctx, tracking_id, OrderStatus::Approved)?;
    assert_eq!(
        SagaStatus::Processing,
        ctx.saga_repository.find_by_id(saga_id)?.unwrap().status(),
    );

    // Step 3: payment request relayed to Kafka (only one, despite the duplicate approval).
    std::thread::sleep(Duration::from_secs(2));
    assert_eq!(1, ctx.relay.process_batch()?);
    assert!(
        ctx.outbox_repository
            .find_by_status(OutboxStatus::Started, 10)?
            .is_empty()
    );

    let payload = consume_one(&ctx.kafka_host, topics::PAYMENT_REQUEST)?;
    let payment_request = PaymentRequestAvroModel::from_avro_bytes(&payload)?;
    assert_eq!(request.saga_id, payment_request[0].saga_id);
    assert_eq!(request.order_id, payment_request[0].order_id);
    assert_eq!(request.customer_id, payment_request[0].customer_id);
    assert_eq!("60.00", payment_request[0].price);

    // Step 4: payment completes -> order PAID, saga SUCCEEDED.
    let reply = PaymentResponseAvroModel {
        id: Uuid::new_v4().to_string(),
        saga_id: request.saga_id.clone(),
        payment_id: Uuid::new_v4().to_string(),
        order_id: request.order_id.clone(),
        customer_id: request.customer_id.clone(),
        price: "60.00".into(),
        payment_status: "COMPLETED".into(),
        failure_messages: vec![],
        created_at: "2026-10-07T10:00:01+00:00".into(),
    };
    produce(
        &ctx.kafka_host,
        topics::PAYMENT_RESPONSE,
        &request.order_id,
        &reply.to_avro_bytes()?,
    )?;

    let paid = wait_for_order_status(&ctx, tracking_id, OrderStatus::Paid)?;
    assert!(paid.failure_messages().is_empty());
    assert_eq!(
        SagaStatus::Succeeded,
        ctx.saga_repository.find_by_id(saga_id)?.unwrap().status(),
    );

    Ok(())
}

#[tokio::test]
async fn test_order_saga_rejected_by_restaurant_ends_cancelled() -> anyhow::Result<()> {
    let ctx = setup().await?;
    let tracking_id = place_order(&ctx)?;

    let request = relay_approval_request(&ctx)?;
    reply_from_restaurant(
        &ctx,
        &request,
        "REJECTED",
        vec!["Insufficient stock for product Pizza: requested 2, available 1".into()],
    )?;

    let cancelled = wait_for_order_status(&ctx, tracking_id, OrderStatus::Canceled)?;
    assert_eq!(
        vec!["Insufficient stock for product Pizza: requested 2, available 1".to_string()],
        cancelled.failure_messages(),
    );
    assert_eq!(
        SagaStatus::Failed,
        ctx.saga_repository
            .find_by_id(Uuid::parse_str(&request.saga_id)?)?
            .unwrap()
            .status(),
    );

    // No payment is requested for a rejected order.
    assert_eq!(0, ctx.relay.process_batch()?);

    Ok(())
}

#[tokio::test]
async fn test_payment_failure_after_approval_requests_restaurant_cancellation() -> anyhow::Result<()>
{
    let ctx = setup().await?;
    let tracking_id = place_order(&ctx)?;

    let request = relay_approval_request(&ctx)?;
    let saga_id = Uuid::parse_str(&request.saga_id)?;
    reply_from_restaurant(&ctx, &request, "APPROVED", vec![])?;
    wait_for_order_status(&ctx, tracking_id, OrderStatus::Approved)?;
    assert_eq!(1, ctx.relay.process_batch()?);

    // The payment fails (delivered twice to prove idempotency).
    let reply = PaymentResponseAvroModel {
        id: Uuid::new_v4().to_string(),
        saga_id: request.saga_id.clone(),
        payment_id: Uuid::new_v4().to_string(),
        order_id: request.order_id.clone(),
        customer_id: request.customer_id.clone(),
        price: "60.00".into(),
        payment_status: "FAILED".into(),
        failure_messages: vec!["Insufficient credit".into()],
        created_at: "2026-10-07T10:00:01+00:00".into(),
    };
    for _ in 0..2 {
        produce(
            &ctx.kafka_host,
            topics::PAYMENT_RESPONSE,
            &request.order_id,
            &reply.to_avro_bytes()?,
        )?;
    }

    let cancelled = wait_for_order_status(&ctx, tracking_id, OrderStatus::Canceled)?;
    assert_eq!(
        vec!["Insufficient credit".to_string()],
        cancelled.failure_messages()
    );
    assert_eq!(
        SagaStatus::Failed,
        ctx.saga_repository.find_by_id(saga_id)?.unwrap().status(),
    );

    // Compensation: exactly one cancel request, so the restaurant gives the stock back.
    std::thread::sleep(Duration::from_secs(2));
    assert_eq!(1, ctx.relay.process_batch()?);
    assert!(
        ctx.outbox_repository
            .find_by_status(OutboxStatus::Started, 10)?
            .is_empty()
    );

    let payload = consume_one(&ctx.kafka_host, topics::RESTAURANT_APPROVAL_CANCEL_REQUEST)?;
    let cancel_request = RestaurantApprovalCancelRequestAvroModel::from_avro_bytes(&payload)?;
    assert_eq!(request.saga_id, cancel_request[0].saga_id);
    assert_eq!(request.order_id, cancel_request[0].order_id);
    assert_eq!(request.products, cancel_request[0].products);

    Ok(())
}
