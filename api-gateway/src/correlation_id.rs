use axum::{extract::Request, middleware::Next, response::Response};
use http::HeaderValue;
use tracing::Instrument;
use uuid::Uuid;

/// Same header the services read in their own `CorrelationIdMiddleware`.
pub const CORRELATION_ID: &str = "x-correlation-id";

/// Makes sure every request carries a correlation id before it is forwarded,
/// and echoes it back to the caller when the upstream response did not.
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

    let span = tracing::info_span!(
        "gateway_request",
        correlation_id = correlation_id.to_str().unwrap_or_default()
    );
    let mut response = next.run(request).instrument(span).await;
    response
        .headers_mut()
        .entry(CORRELATION_ID)
        .or_insert(correlation_id);
    response
}
