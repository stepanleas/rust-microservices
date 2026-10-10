use std::{env, time::Duration};

use anyhow::{Context, Result};

use crate::circuit_breaker::CircuitBreakerConfig;

/// Every downstream service the gateway can route to, with the URL used when
/// `MICROSERVICE_<NAME>_URL` is not set (the ports the services use locally).
const SERVICES: &[(&str, &str)] = &[
    ("catalog", "http://localhost:8080"),
    ("order", "http://localhost:8081"),
    ("customer", "http://localhost:8082"),
    ("restaurant", "http://localhost:8083"),
    ("payment", "http://localhost:8084"),
];

#[derive(Clone, Debug)]
pub struct Settings {
    pub http_url: String,
    pub connect_timeout: Duration,
    pub response_timeout: Duration,
    pub circuit_breaker: CircuitBreakerConfig,
    pub routes: Vec<RouteConfig>,
}

/// `/shop/<name>/<rest>` is forwarded to `<upstream>/<rest>`.
#[derive(Clone, Debug)]
pub struct RouteConfig {
    pub name: String,
    pub upstream: String,
}

impl Settings {
    pub fn from_env() -> Result<Self> {
        let routes = SERVICES
            .iter()
            .map(|(name, default_url)| {
                let key = format!("MICROSERVICE_{}_URL", name.to_uppercase());
                let upstream = env::var(&key).unwrap_or_else(|_| default_url.to_string());
                RouteConfig {
                    name: name.to_string(),
                    upstream: upstream.trim_end_matches('/').to_string(),
                }
            })
            .collect();

        Ok(Self {
            http_url: env::var("HTTP_URL").unwrap_or_else(|_| "0.0.0.0:8070".to_string()),
            connect_timeout: Duration::from_millis(env_or("CONNECT_TIMEOUT_MS", 1_000)?),
            response_timeout: Duration::from_millis(env_or("RESPONSE_TIMEOUT_MS", 10_000)?),
            circuit_breaker: CircuitBreakerConfig {
                sliding_window_size: env_or("CB_SLIDING_WINDOW_SIZE", 10)?,
                permitted_calls_in_half_open: env_or("CB_PERMITTED_CALLS_IN_HALF_OPEN", 2)?,
                failure_rate_threshold: env_or("CB_FAILURE_RATE_THRESHOLD", 50)?,
                wait_duration_in_open: Duration::from_millis(env_or("CB_WAIT_IN_OPEN_MS", 10_000)?),
            },
            routes,
        })
    }
}

fn env_or<T>(key: &str, default: T) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match env::var(key) {
        Ok(value) => value
            .parse()
            .with_context(|| format!("invalid value for {key}")),
        Err(_) => Ok(default),
    }
}
