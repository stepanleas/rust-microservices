use crate::dtos::OrderDto;
use crate::ports::out::repositories::OrderRepository;
use crate::queries::TrackOrderQuery;
use shared::error::{AppError, AppResult};
use std::sync::Arc;

/// Returns the current state of an order by its tracking id.
pub struct TrackOrder {
    order_repository: Arc<dyn OrderRepository>,
}

impl TrackOrder {
    pub fn new(order_repository: Arc<dyn OrderRepository>) -> Self {
        Self { order_repository }
    }

    pub fn execute(&self, query: TrackOrderQuery) -> AppResult<OrderDto> {
        let order = self
            .order_repository
            .find_by_tracking_id(query.tracking_id())?
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "Could not find order by tracking id: {}",
                    query.tracking_id()
                ))
            })?;

        Ok(OrderDto::from(&order))
    }
}
