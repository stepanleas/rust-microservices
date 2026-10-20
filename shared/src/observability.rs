//! Telemetry of a process: logs ([`crate::logging`]), Prometheus metrics and OpenTelemetry traces.
//!
//! - Metrics: every `metrics::counter!`/`gauge!`/`histogram!` of the process, rendered for
//!   Prometheus by [`render`] (served as `GET /metrics`). Histograms named `*_seconds` get the
//!   [`LATENCY_BUCKETS`].
//! - Traces: exported over OTLP/HTTP when `OTEL_EXPORTER_OTLP_ENDPOINT` is set and not empty
//!   (in Kubernetes, the Alloy agent of the node). The standard `OTEL_*` variables apply, e.g.
//!   `OTEL_TRACES_SAMPLER=parentbased_traceidratio` + `OTEL_TRACES_SAMPLER_ARG=0.1` to keep 10%
//!   of the traces during a load test. [`crate::trace_context`] propagates them.

use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::sync::OnceLock;
use std::time::Duration;

/// Histogram buckets (seconds) from 1 ms to 1 min: HTTP requests, Kafka handlers, the outbox
/// relay and whole sagas.
pub const LATENCY_BUCKETS: &[f64] = &[
    0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0,
];

/// How often the recorder drains histogram samples into their buckets.
const UPKEEP_INTERVAL: Duration = Duration::from_secs(5);

static PROMETHEUS: OnceLock<PrometheusHandle> = OnceLock::new();

/// Keeps the trace pipeline alive; dropping it flushes the spans not exported yet.
#[must_use = "dropping it stops the trace export"]
pub struct Telemetry {
    tracer_provider: Option<SdkTracerProvider>,
}

impl Drop for Telemetry {
    fn drop(&mut self) {
        if let Some(provider) = self.tracer_provider.take()
            && let Err(error) = provider.shutdown()
        {
            tracing::warn!("Failed to flush the remaining spans: {error}");
        }
    }
}

/// Installs logging, the metrics recorder and (when configured) the trace export. Call once,
/// first thing in `main`, instead of [`crate::logging::init`], and keep the result until exit.
///
/// `service_name` names the traces unless `OTEL_SERVICE_NAME` is set.
pub fn init(service_name: &str) -> Telemetry {
    let tracer_provider = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .is_ok_and(|endpoint| !endpoint.is_empty())
        .then(|| tracer_provider(service_name))
        .transpose();

    let (tracer_provider, trace_error) = match tracer_provider {
        Ok(provider) => (provider, None),
        Err(error) => (None, Some(error)),
    };
    let otel_layer = tracer_provider.as_ref().map(|provider| {
        Box::new(tracing_opentelemetry::layer().with_tracer(provider.tracer("shared")))
            as crate::logging::BoxedLayer
    });
    crate::logging::init_with(otel_layer);

    match (&tracer_provider, trace_error) {
        (Some(_), _) => tracing::info!("Exporting traces over OTLP"),
        (None, Some(error)) => tracing::error!("Trace export disabled: {error:#}"),
        (None, None) => tracing::debug!("No OTEL_EXPORTER_OTLP_ENDPOINT, no trace export"),
    }

    if let Err(error) = install_prometheus() {
        tracing::error!("Metrics disabled: {error:#}");
    }

    Telemetry { tracer_provider }
}

/// The current value of every metric in the Prometheus text format.
pub fn render() -> String {
    PROMETHEUS
        .get()
        .map(PrometheusHandle::render)
        .unwrap_or_default()
}

fn install_prometheus() -> anyhow::Result<()> {
    let handle = PrometheusBuilder::new()
        .set_buckets_for_metric(Matcher::Suffix("_seconds".to_string()), LATENCY_BUCKETS)?
        .install_recorder()?;

    let upkeep = handle.clone();
    std::thread::Builder::new()
        .name("metrics-upkeep".to_string())
        .spawn(move || {
            loop {
                std::thread::sleep(UPKEEP_INTERVAL);
                upkeep.run_upkeep();
            }
        })?;

    PROMETHEUS
        .set(handle)
        .map_err(|_| anyhow::anyhow!("metrics already initialised"))
}

fn tracer_provider(service_name: &str) -> anyhow::Result<SdkTracerProvider> {
    let mut resource = Resource::builder();
    if std::env::var_os("OTEL_SERVICE_NAME").is_none() {
        resource = resource.with_service_name(service_name.to_string());
    }
    let resource = resource.build();

    // The exporter's blocking HTTP client must not be created on an async runtime's thread
    // (main runs on one), so build it on a thread of its own.
    let exporter = std::thread::spawn(|| {
        opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .build()
    })
    .join()
    .map_err(|_| anyhow::anyhow!("building the OTLP exporter panicked"))??;

    Ok(SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build())
}
