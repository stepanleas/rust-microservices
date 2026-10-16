use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Instant;

/// Path of the Prometheus endpoint; not counted by [`record`].
pub const METRICS_PATH: &str = "/metrics";

/// `GET /metrics`: every metric of the process in the Prometheus text format.
pub async fn metrics_endpoint() -> Response {
    (
        [(http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        shared::observability::render(),
    )
        .into_response()
}

/// Records the same RED metrics as the services' `HttpMetrics` middleware:
/// `http_requests_total{method, route, status}` and `http_request_duration_seconds`.
/// `route` is the route template, e.g. `/shop/{service}/{*rest}`; the per-upstream view is
/// `gateway_upstream_*` (see `proxy`).
pub async fn record(request: Request, next: Next) -> Response {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|path| path.as_str().to_string())
        .unwrap_or_else(|| "unmatched".to_string());
    if route == METRICS_PATH {
        return next.run(request).await;
    }
    let method = request.method().to_string();
    let started = Instant::now();

    let response = next.run(request).await;

    let labels = [
        ("method", method),
        ("route", route),
        ("status", response.status().as_u16().to_string()),
    ];
    metrics::counter!("http_requests_total", &labels).increment(1);
    metrics::histogram!("http_request_duration_seconds", &labels)
        .record(started.elapsed().as_secs_f64());

    response
}
