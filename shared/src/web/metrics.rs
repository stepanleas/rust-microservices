use actix_web::{
    Error, HttpResponse,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
};
use futures::future::{LocalBoxFuture, Ready, ok};
use std::task::{Context, Poll};
use std::time::Instant;

/// Path of the Prometheus endpoint ([`metrics_endpoint`]); not counted by [`HttpMetrics`].
pub const METRICS_PATH: &str = "/metrics";

/// `GET /metrics`: every metric of the process in the Prometheus text format.
pub async fn metrics_endpoint() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(crate::observability::render())
}

/// Records the RED metrics of every request:
///
/// - `http_requests_total{method, route, status}`
/// - `http_request_duration_seconds{method, route, status}` (histogram)
///
/// `route` is the route template (`/api/orders/{tracking_id}`), not the path, which keeps the
/// number of series bounded; requests no route matched share `route="unmatched"`.
pub struct HttpMetrics;

impl<S, B> Transform<S, ServiceRequest> for HttpMetrics
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = HttpMetricsInner<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(HttpMetricsInner { service })
    }
}

pub struct HttpMetricsInner<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for HttpMetricsInner<S>
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
        let route = req.match_pattern();
        if route.as_deref() == Some(METRICS_PATH) {
            return Box::pin(self.service.call(req));
        }
        let route = route.unwrap_or_else(|| "unmatched".to_string());
        let method = req.method().to_string();
        let started = Instant::now();
        let fut = self.service.call(req);

        Box::pin(async move {
            let result = fut.await;
            let status = match &result {
                Ok(response) => response.status(),
                Err(error) => error.as_response_error().status_code(),
            };
            let labels = [
                ("method", method),
                ("route", route),
                ("status", status.as_u16().to_string()),
            ];
            metrics::counter!("http_requests_total", &labels).increment(1);
            metrics::histogram!("http_request_duration_seconds", &labels)
                .record(started.elapsed().as_secs_f64());

            result
        })
    }
}
