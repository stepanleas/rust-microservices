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
}

impl Settings {
    pub fn new() -> Self {
        Self {
            environment: env::var("ENVIRONMENT").unwrap_or("development".to_string()),
            http_url: env::var("HTTP_URL").unwrap_or("127.0.0.1:8080".to_string()),
            service_name: env::var("SERVICE_NAME").unwrap_or("catalog_api".to_string()),
            database_url: env::var("DATABASE_URL").unwrap_or({
                "postgres://postgres:postgres@localhost:5432/catalog_db".to_string()
            }),
            db_max_connections: parse_env("DB_MAX_CONNECTIONS", 16),
            db_connection_timeout: Duration::from_millis(parse_env(
                "DB_CONNECTION_TIMEOUT_MS",
                5000,
            )),
            kafka_host: env::var("KAFKA_HOST").unwrap_or("localhost:9092".to_string()),
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

#[derive(Debug)]
pub enum EnvironmentKind {
    Development,
    Staging,
    Production,
}

impl EnvironmentKind {
    pub fn from_env() -> Result<Self, String> {
        match env::var("ENVIRONMENT")
            .unwrap_or("development".into())
            .to_lowercase()
            .as_str()
        {
            "development" => Ok(Self::Development),
            "staging" => Ok(Self::Staging),
            "production" => Ok(Self::Production),
            other => Err(format!("Unsupported environment: {other}")),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EnvironmentKind::Development => "development",
            EnvironmentKind::Staging => "staging",
            EnvironmentKind::Production => "production",
        }
    }
}
