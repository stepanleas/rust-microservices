use std::env;
use std::str::FromStr;
use std::time::Duration;

/// Configuration of the order service, read from the environment (see `.env.example`).
#[derive(Debug, Clone)]
pub struct Settings {
    pub environment: String,
    pub http_url: String,
    pub service_name: String,
    pub database_url: String,
    pub db_max_connections: u32,
    pub db_connection_timeout_ms: u64,
    pub kafka_host: String,
    pub kafka_group_id: String,
    pub outbox_poll_interval_ms: u64,
    pub outbox_batch_size: i64,
    pub outbox_max_attempts: i32,
}

fn var(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

fn parsed<T: FromStr>(name: &str, default: T) -> T {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

impl Settings {
    pub fn new() -> Self {
        Self {
            environment: var("ENVIRONMENT", "development"),
            http_url: var("HTTP_URL", "127.0.0.1:8081"),
            service_name: var("SERVICE_NAME", "order_api"),
            database_url: var(
                "DATABASE_URL",
                "postgres://postgres:postgres@localhost:5432/order_db",
            ),
            db_max_connections: parsed("DB_MAX_CONNECTIONS", 16),
            db_connection_timeout_ms: parsed("DB_CONNECTION_TIMEOUT_MS", 5000),
            kafka_host: var("KAFKA_HOST", "localhost:9092"),
            kafka_group_id: var("KAFKA_GROUP_ID", "order-service-group"),
            outbox_poll_interval_ms: parsed("OUTBOX_POLL_INTERVAL_MS", 1000),
            outbox_batch_size: parsed("OUTBOX_BATCH_SIZE", 100),
            outbox_max_attempts: parsed("OUTBOX_MAX_ATTEMPTS", 10),
        }
    }

    pub fn db_settings(&self) -> shared::db::DbSettings {
        shared::db::DbSettings {
            url: self.database_url.clone(),
            max_connections: self.db_max_connections,
            connection_timeout: Duration::from_millis(self.db_connection_timeout_ms),
        }
    }

    pub fn relay_settings(&self) -> shared::outbox::relay::RelaySettings {
        shared::outbox::relay::RelaySettings {
            poll_interval: Duration::from_millis(self.outbox_poll_interval_ms),
            batch_size: self.outbox_batch_size,
            max_attempts: self.outbox_max_attempts,
        }
    }
}
