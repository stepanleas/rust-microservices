use crate::trace_context::{self, TRACEPARENT};
use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
};
use futures::future::{LocalBoxFuture, Ready, ok};
use std::task::{Context, Poll};
use tracing::field::Empty;
use tracing::{Instrument, Level};
use uuid::Uuid;

pub const CORRELATION_ID: &str = "X-Correlation-ID";

/// Puts each request in an `http_request` span with its correlation id (taken from the
/// `X-Correlation-ID` header or generated), continuing the caller's trace (`traceparent`).
pub struct CorrelationIdMiddleware;

impl<S, B> Transform<S, ServiceRequest> for CorrelationIdMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = CorrelationIdMiddlewareInner<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(CorrelationIdMiddlewareInner { service })
    }
}

pub struct CorrelationIdMiddlewareInner<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for CorrelationIdMiddlewareInner<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let correlation_id = req
            .headers()
            .get(CORRELATION_ID)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // The route template, not the raw path, so traces group by endpoint.
        let route = req
            .match_pattern()
            .unwrap_or_else(|| "unmatched".to_string());
        macro_rules! request_span {
            ($level:expr) => {
                tracing::span!(
                    $level,
                    "http_request",
                    %correlation_id,
                    trace_id = Empty,
                    otel.name = format!("{} {}", req.method(), route),
                    otel.kind = "server",
                    http.response.status_code = Empty,
                    otel.status_code = Empty,
                )
            };
        }
        // Probes and scrapes come every few seconds per pod: at DEBUG (filtered out by default)
        // they don't each become a trace.
        let span = if is_infrastructure(&route) {
            request_span!(Level::DEBUG)
        } else {
            request_span!(Level::INFO)
        };
        let traceparent = req
            .headers()
            .get(TRACEPARENT)
            .and_then(|value| value.to_str().ok());
        trace_context::continue_trace(&span, traceparent);

        req.extensions_mut().insert(correlation_id);

        // A span guard must not be held across `.await`; instrumenting the future enters the span
        // on every poll instead, so everything the handler logs carries the correlation id.
        let fut = span.in_scope(|| self.service.call(req));
        let request_span = span.clone();
        Box::pin(
            async move {
                let result = fut.await;
                if let Ok(response) = &result {
                    let status = response.status();
                    request_span.record("http.response.status_code", status.as_u16());
                    if status.is_server_error() {
                        request_span.record("otel.status_code", "ERROR");
                    }
                }
                result
            }
            .instrument(span),
        )
    }
}

/// Health probes and the Prometheus endpoint.
fn is_infrastructure(route: &str) -> bool {
    route.starts_with("/api/health/") || route == "/metrics"
}
