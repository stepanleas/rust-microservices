//! Runs the payment service's saga participant against real Postgres and Kafka
//! containers: `payment-request` -> payments + outbox_messages -> `payment-response`.

use application::outbox::OutboxMessageType;
use application::ports::out::repositories::PaymentRepository;
use domain::aggregates::payment::Payment;
use domain::enums::PaymentStatus;
use infrastructure::DbPool;
use infrastructure::postgres::PostgresPaymentRepository;
use kafka::client::{FetchOffset, GroupOffsetStorage, KafkaClient};
use kafka::consumer::Consumer;
use kafka::producer::{Producer, Record};
use rust_decimal::Decimal;
use shared::contracts::AvroMessage;
use shared::contracts::payment::{PaymentRequestAvroModel, PaymentResponseAvroModel};
use shared::contracts::topics::{PAYMENT_REQUEST, PAYMENT_RESPONSE};
use shared::domain::value_objects::OrderId;
use shared::kafka::RetryPolicy;
use shared::outbox::OutboxStatus;
use shared::outbox::ports::OutboxRepository;
use shared::outbox::postgres::PostgresOutboxRepository;
use starter::config::Settings;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::kafka::apache::{KAFKA_PORT, Kafka};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

struct TestContext {
    _postgres: ContainerAsync<Postgres>,
    _kafka: ContainerAsync<Kafka>,
    pool: DbPool,
    kafka_host: String,
}

async fn setup_context() -> anyhow::Result<TestContext> {
    let postgres = Postgres::default().start().await?;
    let postgres_port = postgres.get_host_port_ipv4(5432).await?;
    let mut db_settings = settings("127.0.0.1:1");
    db_settings.database_url =
        format!("postgres://postgres:postgres@127.0.0.1:{postgres_port}/postgres");
    let pool = tokio::task::spawn_blocking(move || starter::wiring::connect_database(&db_settings))
        .await??;

    let kafka = Kafka::default().start().await?;
    let kafka_host = format!("127.0.0.1:{}", kafka.get_host_port_ipv4(KAFKA_PORT).await?);

    // `payment-response` is deliberately not created here: the outbox relay's
    // lazily created producer has to create it (and retry until it is ready).
    wait_for_topics(&kafka_host, &[PAYMENT_REQUEST])?;

    Ok(TestContext {
        _postgres: postgres,
        _kafka: kafka,
        pool,
        kafka_host,
    })
}

/// Topics are auto-created by the broker on the first metadata request.
fn wait_for_topics(host: &str, topics: &[&str]) -> anyhow::Result<()> {
    let mut client = KafkaClient::new(vec![host.to_string()]);
    let deadline = Instant::now() + Duration::from_secs(30);

    loop {
        let _ = client.load_metadata(topics);
        let ready = topics.iter().all(|topic| {
            client
                .topics()
                .partitions(topic)
                .map(|partitions| !partitions.is_empty())
                .unwrap_or(false)
        });

        if ready {
            return Ok(());
        }
        anyhow::ensure!(Instant::now() < deadline, "topics {topics:?} not ready");
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn settings(kafka_host: &str) -> Settings {
    Settings {
        environment: "test".into(),
        http_url: "127.0.0.1:0".into(),
        service_name: "payment_api".into(),
        database_url: String::new(),
        db_max_connections: 4,
        db_connection_timeout: Duration::from_secs(5),
        kafka_host: kafka_host.to_string(),
        outbox_poll_interval: Duration::from_millis(100),
        outbox_batch_size: 100,
        outbox_max_attempts: 50,
    }
}

fn payment_request(
    saga_id: Uuid,
    order_id: Uuid,
    customer_id: Uuid,
    price: &str,
) -> PaymentRequestAvroModel {
    PaymentRequestAvroModel {
        id: Uuid::new_v4().to_string(),
        saga_id: saga_id.to_string(),
        order_id: order_id.to_string(),
        customer_id: customer_id.to_string(),
        price: price.into(),
        created_at: "2026-10-07T12:00:00+00:00".into(),
    }
}

fn produce_payment_requests(
    host: &str,
    requests: Vec<PaymentRequestAvroModel>,
) -> anyhow::Result<()> {
    let mut producer = Producer::from_hosts(vec![host.to_string()])
        .with_ack_timeout(Duration::from_secs(1))
        .create()?;

    for request in requests {
        let bytes = request.to_avro_bytes()?;
        producer.send(&Record::from_key_value(
            PAYMENT_REQUEST,
            request.order_id.as_bytes(),
            bytes.as_slice(),
        ))?;
    }

    Ok(())
}

fn wait_for_payment(pool: &DbPool, order_id: Uuid) -> anyhow::Result<Payment> {
    let repository = PostgresPaymentRepository::new(pool);
    let deadline = Instant::now() + Duration::from_secs(60);

    loop {
        if let Some(payment) = repository.find_by_order_id(OrderId::from_uuid(order_id))? {
            return Ok(payment);
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            "payment for order {order_id} was not created"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn consume_payment_responses(
    host: &str,
    expected: usize,
) -> anyhow::Result<HashMap<String, PaymentResponseAvroModel>> {
    let mut consumer = Consumer::from_hosts(vec![host.to_string()])
        .with_topic(PAYMENT_RESPONSE.to_string())
        .with_group(format!("test-{}", Uuid::new_v4()))
        .with_fallback_offset(FetchOffset::Earliest)
        .with_offset_storage(GroupOffsetStorage::Kafka.into())
        .create()?;

    let mut responses = HashMap::new();
    let deadline = Instant::now() + Duration::from_secs(30);

    while responses.len() < expected {
        anyhow::ensure!(Instant::now() < deadline, "payment responses not received");

        for message_set in consumer.poll()?.iter() {
            for message in message_set.messages() {
                assert!(!message.key.is_empty(), "message must be keyed by order id");

                for response in PaymentResponseAvroModel::from_avro_bytes(message.value)? {
                    assert_eq!(response.order_id.as_bytes(), message.key);
                    responses.insert(response.order_id.clone(), response);
                }
            }
        }
    }

    Ok(responses)
}

#[tokio::test]
async fn test_payment_request_is_paid_and_answered_through_the_outbox() -> anyhow::Result<()> {
    let ctx = setup_context().await?;

    let completed_saga_id = Uuid::new_v4();
    let completed_order_id = Uuid::new_v4();
    let failed_saga_id = Uuid::new_v4();
    let failed_order_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();

    // The completed request is delivered twice to verify idempotency.
    produce_payment_requests(
        &ctx.kafka_host,
        vec![
            payment_request(completed_saga_id, completed_order_id, customer_id, "150.00"),
            payment_request(completed_saga_id, completed_order_id, customer_id, "150.00"),
            payment_request(failed_saga_id, failed_order_id, customer_id, "0"),
        ],
    )?;

    let hosts = vec![ctx.kafka_host.clone()];
    let handler = Arc::new(starter::wiring::build_message_handler(&ctx.pool));
    std::thread::spawn(move || {
        shared::kafka::listen(
            hosts,
            "payment-test-group".into(),
            handler,
            RetryPolicy::default(),
        )
    });

    let completed_payment = wait_for_payment(&ctx.pool, completed_order_id)?;
    assert_eq!(PaymentStatus::Completed, completed_payment.payment_status());
    assert_eq!(Decimal::new(15000, 2), completed_payment.price());
    assert!(completed_payment.failure_messages().is_empty());

    let failed_payment = wait_for_payment(&ctx.pool, failed_order_id)?;
    assert_eq!(PaymentStatus::Failed, failed_payment.payment_status());
    assert_eq!(
        vec!["Total price must be greater than zero!".to_string()],
        failed_payment.failure_messages(),
    );

    // Give the listener time to process the duplicate (it is the second message of the batch).
    std::thread::sleep(Duration::from_secs(1));

    let outbox_repository = PostgresOutboxRepository::<OutboxMessageType>::new(&ctx.pool);
    let pending = outbox_repository.find_by_status(OutboxStatus::Started, 100)?;
    assert_eq!(
        2,
        pending.len(),
        "duplicate request must not create a message"
    );

    // Broker unreachable: nothing is published, the attempt is recorded and the
    // messages stay STARTED so they are retried later.
    let offline_relay = starter::wiring::build_outbox_relay(&settings("127.0.0.1:1"), &ctx.pool);
    assert_eq!(0, offline_relay.process_batch()?);
    let pending = outbox_repository.find_by_status(OutboxStatus::Started, 100)?;
    assert_eq!(2, pending.len());
    assert!(pending.iter().all(|message| message.attempts() == 1));

    // Broker reachable: the relay keeps retrying on every tick until the
    // `payment-response` topic is ready and both messages are published.
    let relay = starter::wiring::build_outbox_relay(&settings(&ctx.kafka_host), &ctx.pool);
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut published = 0;
    while published < 2 {
        anyhow::ensure!(
            Instant::now() < deadline,
            "outbox messages were not published"
        );
        published += relay.process_batch()?;
        std::thread::sleep(Duration::from_millis(200));
    }
    assert_eq!(2, published);
    assert!(
        outbox_repository
            .find_by_status(OutboxStatus::Started, 100)?
            .is_empty()
    );
    assert_eq!(
        2,
        outbox_repository
            .find_by_status(OutboxStatus::Completed, 100)?
            .len()
    );

    let responses = consume_payment_responses(&ctx.kafka_host, 2)?;

    let completed = &responses[&completed_order_id.to_string()];
    assert!(Uuid::parse_str(&completed.id).is_ok());
    assert_eq!(completed_saga_id.to_string(), completed.saga_id);
    assert_eq!(
        completed_payment.id().as_uuid().to_string(),
        completed.payment_id
    );
    assert_eq!(customer_id.to_string(), completed.customer_id);
    assert_eq!("150.00", completed.price);
    assert_eq!("COMPLETED", completed.payment_status);
    assert!(completed.failure_messages.is_empty());
    assert!(!completed.created_at.is_empty());

    let failed = &responses[&failed_order_id.to_string()];
    assert_eq!(failed_saga_id.to_string(), failed.saga_id);
    assert_eq!("FAILED", failed.payment_status);
    assert_eq!(
        vec!["Total price must be greater than zero!".to_string()],
        failed.failure_messages
    );

    Ok(())
}
