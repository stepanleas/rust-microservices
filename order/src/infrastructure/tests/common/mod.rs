use infrastructure::DbPool;
use infrastructure::MIGRATIONS;
use shared::db::{DbSettings, connect};
use std::time::Duration;
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

pub type PostgresContainer = ContainerAsync<Postgres>;

/// Starts a throwaway Postgres container and returns a migrated connection pool.
/// The container is removed when dropped, so keep it alive alongside the pool.
pub async fn start_postgres() -> anyhow::Result<(PostgresContainer, DbPool)> {
    let container = Postgres::default().start().await?;
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let settings = DbSettings {
        url,
        max_connections: 4,
        connection_timeout: Duration::from_secs(5),
    };
    let pool = tokio::task::spawn_blocking(move || connect(&settings, MIGRATIONS)).await??;

    Ok((container, pool))
}
