//! Log output of the services, through `tracing`.
//!
//! - `RUST_LOG` filters as usual (default `info`).
//! - `LOG_FORMAT=json` writes one JSON object per line with the fields of the enclosing spans
//!   (`correlation_id` of the HTTP request, `saga_id` of an outbox message, `topic`/`key` of a
//!   Kafka message, `trace_id` with the `observability` feature), for Loki in Kubernetes.
//!   Anything else gives readable output.
//!
//! Records of the `log` crate (actix's `Logger`, diesel, r2d2) are forwarded to `tracing`.

use std::io::IsTerminal;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry};

pub(crate) type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync>;

/// Installs the global subscriber. Call once, first thing in `main`.
pub fn init() {
    init_with(None);
}

/// [`init`] plus another layer that sees the same (filtered) spans and events, e.g. the
/// OpenTelemetry layer of [`crate::observability`].
pub(crate) fn init_with(extra: Option<BoxedLayer>) {
    subscriber(extra).init();
}

/// The subscriber [`init_with`] installs.
pub(crate) fn subscriber(extra: Option<BoxedLayer>) -> impl tracing::Subscriber + Send + Sync {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let json = std::env::var("LOG_FORMAT").is_ok_and(|format| format.eq_ignore_ascii_case("json"));

    let output: BoxedLayer = if json {
        tracing_subscriber::fmt::layer()
            .json()
            .flatten_event(true)
            .with_current_span(false)
            .with_span_list(true)
            .boxed()
    } else {
        // No colour codes when the output goes to a file or `docker logs`.
        tracing_subscriber::fmt::layer()
            .with_ansi(std::io::stdout().is_terminal())
            .boxed()
    };

    let layers: Vec<BoxedLayer> = std::iter::once(output).chain(extra).collect();
    tracing_subscriber::registry().with(layers.with_filter(filter))
}
