use axum::{extract::Request, middleware::Next, response::Response};
use http::HeaderValue;
use shared::trace_context::{self, TRACEPARENT};
use tracing::field::Empty;
use tracing::{Instrument, Level};
use uuid::Uuid;

/// Same header the services read in their own `CorrelationIdMiddleware`.
pub const CORRELATION_ID: &str = "x-correlation-id";

/// Makes sure every request carries a correlation id before it is forwarded,
/// and echoes it back to the caller when the upstream response did not. Also
/// puts the request in a `gateway_request` span of the caller's trace.
pub async fn correlation_id(mut request: Request, next: Next) -> Response {
    let correlation_id = match request.headers().get(CORRELATION_ID) {
        Some(value) => {
            tracing::debug!(?value, "correlation id found in request");
            value.clone()
        }
        None => {
            let generated = HeaderValue::from_str(&Uuid::new_v4().to_string())
                .expect("a UUID is a valid header value");
            tracing::debug!(?generated, "correlation id generated");
            request
                .headers_mut()
                .insert(CORRELATION_ID, generated.clone());
            generated
        }
    };

    let path = request.uri().path();
    macro_rules! request_span {
        ($level:expr) => {
            tracing::span!(
                $level,
                "gateway_request",
                correlation_id = correlation_id.to_str().unwrap_or_default(),
                trace_id = Empty,
                otel.name = format!("{} {}", request.method(), span_route(path)),
                otel.kind = "server",
                http.response.status_code = Empty,
                otel.status_code = Empty,
            )
        };
    }
    // Probes and scrapes come every few seconds: at DEBUG (filtered out by default) they don't
    // each become a trace.
    let span = if path.starts_with("/api/health/") || path == crate::http_metrics::METRICS_PATH {
        request_span!(Level::DEBUG)
    } else {
        request_span!(Level::INFO)
    };
    // Continues the caller's trace, or starts one; `proxy` passes it on to the upstream.
    let traceparent = request
        .headers()
        .get(TRACEPARENT)
        .and_then(|value| value.to_str().ok());
    trace_context::continue_trace(&span, traceparent);

    let mut response = next.run(request).instrument(span.clone()).await;
    span.record("http.response.status_code", response.status().as_u16());
    if response.status().is_server_error() {
        span.record("otel.status_code", "ERROR");
    }
    response
        .headers_mut()
        .entry(CORRELATION_ID)
        .or_insert(correlation_id);
    response
}

/// `/shop/{service}` part of the path, so span names don't contain ids.
fn span_route(path: &str) -> &str {
    match path.strip_prefix("/shop/") {
        Some(rest) => {
            let service_end = rest
                .find('/')
                .map_or(path.len(), |end| "/shop/".len() + end);
            &path[..service_end]
        }
        None => path,
    }
}

#[cfg(test)]
mod tests {
    use super::span_route;

    #[test]
    fn span_route_keeps_only_the_service() {
        assert_eq!("/shop/order", span_route("/shop/order/api/orders/42"));
        assert_eq!("/shop/order", span_route("/shop/order"));
        assert_eq!("/api/health/live", span_route("/api/health/live"));
    }
}
