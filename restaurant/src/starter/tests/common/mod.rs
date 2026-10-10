#![allow(dead_code)]

use infrastructure::{DbPool, MIGRATIONS};
use shared::contracts::AvroMessage;
use shared::contracts::restaurant::{
    RestaurantApprovalCancelRequestAvroModel, RestaurantApprovalProductAvroModel,
    RestaurantApprovalRequestAvroModel,
};
use shared::db::DbSettings;
use shared::kafka::KafkaConsumer;
use std::time::{Duration, Instant};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::kafka::apache::{KAFKA_PORT, Kafka};
use testcontainers_modules::postgres::Postgres;

pub type PostgresContainer = ContainerAsync<Postgres>;
pub type KafkaContainer = ContainerAsync<Kafka>;

pub async fn start_postgres() -> anyhow::Result<(PostgresContainer, DbPool)> {
    let container = Postgres::default().start().await?;
    let port = container.get_host_port_ipv4(5432).await?;
    let settings = DbSettings {
        url: format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres"),
        max_connections: 4,
        connection_timeout: Duration::from_secs(5),
    };

    let pool =
        tokio::task::spawn_blocking(move || shared::db::connect(&settings, MIGRATIONS)).await??;

    Ok((container, pool))
}

/// Same major version as docker-compose and the Helm chart.
const KAFKA_VERSION: &str = "4.3.1";

/// Starts Kafka and returns the bootstrap address reachable from the host.
pub async fn start_kafka() -> anyhow::Result<(KafkaContainer, String)> {
    let container = Kafka::default().with_tag(KAFKA_VERSION).start().await?;
    let host = format!(
        "127.0.0.1:{}",
        container.get_host_port_ipv4(KAFKA_PORT).await?
    );

    Ok((container, host))
}

/// Creates the topics (3 partitions) unless they exist.
pub fn ensure_topics(host: &str, topics: &[&str]) -> anyhow::Result<()> {
    shared::kafka::ensure_topics(&[host.to_string()], topics, 3, Duration::from_secs(30))
}

/// Consumes the first message of `topic` (from the earliest offset).
pub fn consume_one(host: &str, topic: &str) -> anyhow::Result<Vec<u8>> {
    let consumer = KafkaConsumer::new(
        vec![host.to_string()],
        format!("test-{}", uuid::Uuid::new_v4()),
        &[topic],
    )?;

    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if let Some(message) = consumer.poll(Duration::from_millis(500))? {
            return Ok(message.payload);
        }
    }

    anyhow::bail!("no message received on topic {topic}")
}

/// Decodes a single-record Avro container.
pub fn decode_one<T: AvroMessage>(bytes: &[u8]) -> anyhow::Result<T> {
    let mut records = T::from_avro_bytes(bytes)?;
    anyhow::ensure!(
        records.len() == 1,
        "expected one record, got {}",
        records.len()
    );
    Ok(records.remove(0))
}

fn products(products: &[(&str, i32)]) -> Vec<RestaurantApprovalProductAvroModel> {
    products
        .iter()
        .map(|(id, quantity)| RestaurantApprovalProductAvroModel {
            id: (*id).into(),
            quantity: *quantity,
        })
        .collect()
}

pub fn approval_request(
    saga_id: &str,
    order_id: &str,
    items: &[(&str, i32)],
) -> anyhow::Result<Vec<u8>> {
    RestaurantApprovalRequestAvroModel {
        id: uuid::Uuid::new_v4().to_string(),
        saga_id: saga_id.into(),
        order_id: order_id.into(),
        customer_id: uuid::Uuid::new_v4().to_string(),
        price: "30.00".into(),
        products: products(items),
        created_at: "2025-01-01T12:00:00Z".into(),
    }
    .to_avro_bytes()
}

pub fn approval_cancel_request(
    saga_id: &str,
    order_id: &str,
    items: &[(&str, i32)],
) -> anyhow::Result<Vec<u8>> {
    RestaurantApprovalCancelRequestAvroModel {
        id: uuid::Uuid::new_v4().to_string(),
        saga_id: saga_id.into(),
        order_id: order_id.into(),
        products: products(items),
        created_at: "2025-01-01T12:05:00Z".into(),
    }
    .to_avro_bytes()
}
