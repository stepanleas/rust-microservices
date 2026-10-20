//! W3C trace context (`traceparent`) of the current span. It travels in HTTP headers, Kafka
//! record headers and outbox rows, so a single trace follows an order through every service:
//! gateway → order → outbox relay → Kafka → restaurant → ... → payment → order.
//!
//! Without the `observability` feature (no OpenTelemetry layer installed) these are no-ops.

/// Name of the W3C header (HTTP) and record header (Kafka).
pub const TRACEPARENT: &str = "traceparent";

/// The `traceparent` of the current span, `None` outside of a trace.
pub fn current() -> Option<String> {
    #[cfg(feature = "observability")]
    {
        otel::current()
    }
    #[cfg(not(feature = "observability"))]
    {
        None
    }
}

/// Makes `span` a child of the remote span `traceparent` (or the root of a new trace without
/// one) and records the trace id in the span's `trace_id` field, which the JSON logs carry so a
/// log line links to its trace. The span must declare `trace_id = tracing::field::Empty`.
///
/// Call it right after creating the span: a span's parent can't change once it was entered.
pub fn continue_trace(span: &tracing::Span, traceparent: Option<&str>) {
    #[cfg(feature = "observability")]
    otel::continue_trace(span, traceparent);
    #[cfg(not(feature = "observability"))]
    let _ = (span, traceparent);
}

#[cfg(feature = "observability")]
mod otel {
    use super::TRACEPARENT;
    use opentelemetry::propagation::TextMapPropagator;
    use opentelemetry::trace::TraceContextExt;
    use opentelemetry_sdk::propagation::TraceContextPropagator;
    use std::collections::HashMap;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    pub fn current() -> Option<String> {
        let cx = tracing::Span::current().context();
        if !cx.span().span_context().is_valid() {
            return None;
        }

        let mut carrier = HashMap::new();
        TraceContextPropagator::new().inject_context(&cx, &mut carrier);
        carrier.remove(TRACEPARENT)
    }

    pub fn continue_trace(span: &tracing::Span, traceparent: Option<&str>) {
        if let Some(traceparent) = traceparent {
            let carrier = HashMap::from([(TRACEPARENT.to_string(), traceparent.to_string())]);
            // An invalid header gives an empty context: the span then starts a new trace.
            let _ = span.set_parent(TraceContextPropagator::new().extract(&carrier));
        }

        // Builds the OpenTelemetry span, so only now is the trace id known.
        let span_context = span.context().span().span_context().clone();
        if span_context.is_valid() {
            span.record("trace_id", span_context.trace_id().to_string());
        }
    }
}

#[cfg(all(test, feature = "observability"))]
mod tests {
    use super::*;
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_sdk::trace::SdkTracerProvider;

    fn with_tracing(test: impl FnOnce()) {
        let provider = SdkTracerProvider::builder().build();
        let layer = tracing_opentelemetry::layer().with_tracer(provider.tracer("test"));
        let subscriber = crate::logging::subscriber(Some(Box::new(layer)));
        tracing::subscriber::with_default(subscriber, test);
    }

    #[test]
    fn continues_the_remote_trace() {
        with_tracing(|| {
            let remote = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
            let span = tracing::info_span!("work", trace_id = tracing::field::Empty);
            continue_trace(&span, Some(remote));

            let current = span.in_scope(current).expect("inside a trace");
            let parts: Vec<&str> = current.split('-').collect();
            assert_eq!("0af7651916cd43dd8448eb211c80319c", parts[1]);
            assert_ne!(
                "b7ad6b7169203331", parts[2],
                "the span has an id of its own"
            );
        });
    }

    #[test]
    fn starts_a_trace_without_a_parent() {
        with_tracing(|| {
            let span = tracing::info_span!("work", trace_id = tracing::field::Empty);
            continue_trace(&span, None);

            assert!(span.in_scope(current).is_some());
            assert_eq!(None, current(), "no trace outside of a span");
        });
    }
}
