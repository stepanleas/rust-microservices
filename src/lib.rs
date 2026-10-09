pub mod circuit_breaker;
pub mod config;
pub mod correlation_id;
pub mod proxy;

use std::{net::SocketAddr, sync::Arc};

use axum::{
    Json, Router, middleware,
    routing::{any, get},
};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::{config::Settings, proxy::ProxyState};

pub fn app(settings: &Settings) -> anyhow::Result<Router> {
    let state = ProxyState::new(settings)?;

    let shop = Router::new()
        .route("/shop/{service}", any(proxy::forward))
        .route("/shop/{service}/{*rest}", any(proxy::forward))
        .with_state(Arc::clone(&state));

    let health = Router::new()
        .route("/api/health/live", get(health_ok))
        .route("/api/health/ready", get(health_ok));

    Ok(shop
        .merge(health)
        .layer(middleware::from_fn(correlation_id::correlation_id))
        .layer(TraceLayer::new_for_http()))
}

/// Serves until SIGINT/SIGTERM. Takes a bound listener so tests can use port 0.
pub async fn serve(listener: TcpListener, settings: &Settings) -> anyhow::Result<()> {
    for route in &settings.routes {
        tracing::info!("/shop/{}/** -> {}", route.name, route.upstream);
    }
    axum::serve(
        listener,
        app(settings)?.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

async fn health_ok() -> Json<Value> {
    // Same shape as the services, so docker healthchecks can grep for it.
    Json(json!({ "status": "Ok" }))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            signal.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
