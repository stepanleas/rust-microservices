#![allow(dead_code)]

use infrastructure::{DbPool, MIGRATIONS};
use kafka::client::{FetchOffset, GroupOffsetStorage, KafkaClient};
use kafka::consumer::Consumer;
use shared::contracts::AvroMessage;
use shared::contracts::restaurant::{
    RestaurantApprovalCancelRequestAvroModel, RestaurantApprovalProductAvroModel,
    RestaurantApprovalRequestAvroModel,
};
use shared::db::DbSettings;
use std::time::{Duration, Instant};
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
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

/// Starts Kafka and returns the bootstrap address reachable from the host.
pub async fn start_kafka() -> anyhow::Result<(KafkaContainer, String)> {
    let container = Kafka::default().start().await?;
    let host = format!(
        "127.0.0.1:{}",
        container.get_host_port_ipv4(KAFKA_PORT).await?
    );

    Ok((container, host))
}

/// Requests metadata until the (auto-created) topics have partitions.
pub fn ensure_topics(host: &str, topics: &[&str]) -> anyhow::Result<()> {
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
        if Instant::now() > deadline {
            anyhow::bail!("topics {topics:?} were not created in time");
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

/// Consumes the first message of `topic` (from the earliest offset).
pub fn consume_one(host: &str, topic: &str) -> anyhow::Result<Vec<u8>> {
    let mut consumer = Consumer::from_hosts(vec![host.to_string()])
        .with_topic(topic.to_string())
        .with_group(format!("test-{}", uuid::Uuid::new_v4()))
        .with_fallback_offset(FetchOffset::Earliest)
        .with_offset_storage(GroupOffsetStorage::Kafka.into())
        .create()?;

    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        for message_set in consumer.poll()?.iter() {
            if let Some(message) = message_set.messages().first() {
                return Ok(message.value.to_vec());
            }
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
