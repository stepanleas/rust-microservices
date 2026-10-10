use std::env;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Settings {
    pub environment: String,
    pub http_url: String,
    pub service_name: String,
    pub database_url: String,
    pub db_max_connections: u32,
    pub db_connection_timeout: Duration,
    pub kafka_host: String,
    pub outbox_poll_interval: Duration,
    pub outbox_batch_size: i64,
    pub outbox_max_attempts: i32,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            environment: env::var("ENVIRONMENT").unwrap_or("development".to_string()),
            http_url: env::var("HTTP_URL").unwrap_or("127.0.0.1:8083".to_string()),
            service_name: env::var("SERVICE_NAME").unwrap_or("restaurant_api".to_string()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or("postgres://postgres:postgres@localhost:5432/restaurant_db".to_string()),
            db_max_connections: parse_env("DB_MAX_CONNECTIONS", 16),
            db_connection_timeout: Duration::from_millis(parse_env(
                "DB_CONNECTION_TIMEOUT_MS",
                5000,
            )),
            kafka_host: env::var("KAFKA_HOST").unwrap_or("localhost:9092".to_string()),
            outbox_poll_interval: Duration::from_millis(parse_env("OUTBOX_POLL_INTERVAL_MS", 1000)),
            outbox_batch_size: parse_env("OUTBOX_BATCH_SIZE", 100),
            outbox_max_attempts: parse_env("OUTBOX_MAX_ATTEMPTS", 10),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_env<T: std::str::FromStr>(name: &str, default: T) -> T {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
