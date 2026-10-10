//! Log output of the services, through `tracing`.
//!
//! - `RUST_LOG` filters as usual (default `info`).
//! - `LOG_FORMAT=json` writes one JSON object per line with the fields of the enclosing spans
//!   (`correlation_id` of the HTTP request, `saga_id` of an outbox message, `topic`/`key` of a
//!   Kafka message), for Loki in Kubernetes. Anything else gives readable output.
//!
//! Records of the `log` crate (actix's `Logger`, diesel, r2d2) are forwarded to `tracing`.

use std::io::IsTerminal;
use tracing_subscriber::EnvFilter;

/// Installs the global subscriber. Call once, first thing in `main`.
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let json = std::env::var("LOG_FORMAT").is_ok_and(|format| format.eq_ignore_ascii_case("json"));

    if json {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .flatten_event(true)
            .with_current_span(false)
            .with_span_list(true)
            .init();
    } else {
        // No colour codes when the output goes to a file or `docker logs`.
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(std::io::stdout().is_terminal())
            .init();
    }
}
