#![allow(dead_code)]

use chrono::NaiveDateTime;
use diesel::prelude::*;
use infrastructure::schema::outbox_messages;
use infrastructure::{DbPool, MIGRATIONS};
use shared::db::{DbSettings, connect};
use std::time::Duration;
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

pub type PostgresContainer = ContainerAsync<Postgres>;

/// Starts a throwaway Postgres container and returns a migrated connection pool.
/// The container is removed when dropped, so keep it alive alongside the pool.
pub async fn start_postgres() -> anyhow::Result<(PostgresContainer, DbPool)> {
    let container = Postgres::default().start().await?;
    let port = container.get_host_port_ipv4(5432).await?;
    let settings = DbSettings {
        url: format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres"),
        max_connections: 4,
        connection_timeout: Duration::from_secs(5),
    };

    let pool = tokio::task::spawn_blocking(move || connect(&settings, MIGRATIONS)).await??;

    Ok((container, pool))
}

/// Raw view of an `outbox_messages` row (the shared repository has no lookup by id).
#[derive(Debug, Queryable)]
pub struct StoredOutboxMessage {
    pub saga_id: Uuid,
    pub message_type: String,
    pub payload: serde_json::Value,
    pub outbox_status: String,
    pub attempts: i32,
    pub processed_at: Option<NaiveDateTime>,
}

pub fn find_outbox_message(pool: &DbPool, id: Uuid) -> anyhow::Result<Option<StoredOutboxMessage>> {
    let mut connection = pool.get()?;

    Ok(outbox_messages::table
        .find(id)
        .select((
            outbox_messages::saga_id,
            outbox_messages::message_type,
            outbox_messages::payload,
            outbox_messages::outbox_status,
            outbox_messages::attempts,
            outbox_messages::processed_at,
        ))
        .first::<StoredOutboxMessage>(&mut connection)
        .optional()?)
}
