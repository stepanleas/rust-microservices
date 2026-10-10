use super::ApiError;
use crate::error::AppResult;

/// Runs a synchronous use case (diesel, the Kafka producer) on actix's blocking thread pool,
/// so it does not stall the async workers that serve the other requests.
pub async fn blocking<T, F>(use_case: F) -> Result<T, ApiError>
where
    F: FnOnce() -> AppResult<T> + Send + 'static,
    T: Send + 'static,
{
    actix_web::web::block(use_case)
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::from)
}
