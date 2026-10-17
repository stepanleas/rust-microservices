use std::{net::SocketAddr, time::Duration};

use api_gateway::{
    circuit_breaker::CircuitBreakerConfig,
    config::{PoolConfig, RouteConfig, Settings},
};
use axum::{Json, Router, extract::Request, routing::any};
use serde_json::{Value, json};
use tokio::net::TcpListener;

/// Upstream that echoes back what it received.
async fn spawn_echo_upstream() -> String {
    async fn echo(request: Request) -> Json<Value> {
        let header = |name: &str| {
            request
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        Json(json!({
            "method": request.method().as_str(),
            "path": request.uri().path(),
            "query": request.uri().query(),
            "correlation_id": header("x-correlation-id"),
            "forwarded_prefix": header("x-forwarded-prefix"),
            "connection_listed": header("x-drop-me"),
        }))
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, Router::new().fallback(any(echo)))
            .await
            .unwrap();
    });
    format!("http://{addr}")
}

async fn spawn_gateway(routes: Vec<RouteConfig>) -> String {
    let settings = Settings {
        http_url: "127.0.0.1:0".to_string(),
        connect_timeout: Duration::from_millis(200),
        response_timeout: Duration::from_secs(2),
        pool: PoolConfig {
            idle_timeout: Duration::from_secs(30),
            max_idle_per_host: 32,
        },
        circuit_breaker: CircuitBreakerConfig {
            sliding_window_size: 2,
            permitted_calls_in_half_open: 1,
            failure_rate_threshold: 50,
            wait_duration_in_open: Duration::from_secs(60),
        },
        routes,
    };
    let listener = TcpListener::bind(&settings.http_url).await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { api_gateway::serve(listener, &settings).await.unwrap() });
    format!("http://{addr}")
}

fn route(name: &str, upstream: &str) -> RouteConfig {
    RouteConfig {
        name: name.to_string(),
        upstream: upstream.to_string(),
    }
}

#[tokio::test]
async fn strips_prefix_and_forwards_query_and_method() {
    let upstream = spawn_echo_upstream().await;
    let gateway = spawn_gateway(vec![route("catalog", &upstream)]).await;

    let response = reqwest::Client::new()
        .put(format!(
            "{gateway}/shop/catalog/api/products/42?page=2&size=10"
        ))
        .header("connection", "x-drop-me")
        .header("x-drop-me", "secret")
        .body("{}")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert!(response.headers().contains_key("x-response-time"));
    let echoed: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(echoed["method"], "PUT");
    assert_eq!(echoed["path"], "/api/products/42");
    assert_eq!(echoed["query"], "page=2&size=10");
    assert_eq!(echoed["forwarded_prefix"], "/shop/catalog");
    assert_eq!(echoed["connection_listed"], Value::Null);
}

#[tokio::test]
async fn generates_correlation_id_and_returns_it_to_the_caller() {
    let upstream = spawn_echo_upstream().await;
    let gateway = spawn_gateway(vec![route("order", &upstream)]).await;

    let response = reqwest::get(format!("{gateway}/shop/order/api/orders"))
        .await
        .unwrap();

    let returned = response.headers()["x-correlation-id"]
        .to_str()
        .unwrap()
        .to_string();
    let echoed: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(echoed["correlation_id"], returned.as_str());
    assert!(uuid::Uuid::parse_str(&returned).is_ok());
}

#[tokio::test]
async fn keeps_existing_correlation_id() {
    let upstream = spawn_echo_upstream().await;
    let gateway = spawn_gateway(vec![route("customer", &upstream)]).await;

    let response = reqwest::Client::new()
        .get(format!("{gateway}/shop/customer/api/customers"))
        .header("x-correlation-id", "abc-123")
        .send()
        .await
        .unwrap();

    assert_eq!(response.headers()["x-correlation-id"], "abc-123");
    let echoed: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
    assert_eq!(echoed["correlation_id"], "abc-123");
}

#[tokio::test]
async fn unknown_service_is_not_found() {
    let gateway = spawn_gateway(vec![]).await;

    let response = reqwest::get(format!("{gateway}/shop/unknown/anything"))
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn unreachable_upstream_falls_back_and_opens_circuit() {
    // Bind then drop a listener to get a port nobody is listening on.
    let dead = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead_url = format!("http://{}", dead.local_addr().unwrap());
    drop(dead);
    let gateway = spawn_gateway(vec![route("payment", &dead_url)]).await;
    let url = format!("{gateway}/shop/payment/api/payments");

    for _ in 0..3 {
        let response = reqwest::get(&url).await.unwrap();
        assert_eq!(response.status(), 503);
        assert!(response.text().await.unwrap().contains("contact support"));
    }
}

#[tokio::test]
async fn health_endpoints_report_ok() {
    let gateway = spawn_gateway(vec![]).await;

    for path in ["/api/health/live", "/api/health/ready"] {
        let body = reqwest::get(format!("{gateway}{path}"))
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(body.contains("\"status\":\"Ok\""), "{path}: {body}");
    }
}
