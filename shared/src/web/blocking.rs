use super::ApiError;
use crate::error::AppResult;

/// Runs a synchronous use case (diesel, the Kafka producer) on actix's blocking thread pool,
/// so it does not stall the async workers that serve the other requests. The use case runs in
/// the request's span: its logs keep the correlation id, and an outbox message it writes the
/// request's trace.
pub async fn blocking<T, F>(use_case: F) -> Result<T, ApiError>
where
    F: FnOnce() -> AppResult<T> + Send + 'static,
    T: Send + 'static,
{
    let span = tracing::Span::current();
    actix_web::web::block(move || span.in_scope(use_case))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::from)
}
