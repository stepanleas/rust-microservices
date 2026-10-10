use crate::dtos::OrderApprovalDto;
use crate::ports::out::repositories::OrderApprovalRepository;
use shared::domain::value_objects::OrderId;
use shared::error::{AppError, AppResult};
use std::sync::Arc;
use uuid::Uuid;

pub struct GetOrderApproval {
    repository: Arc<dyn OrderApprovalRepository>,
}

impl GetOrderApproval {
    pub fn new(repository: Arc<dyn OrderApprovalRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, order_id: Uuid) -> AppResult<OrderApprovalDto> {
        let approval = self
            .repository
            .find_by_order_id(OrderId::from_uuid(order_id))?
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "Could not find order approval for order id: {order_id}"
                ))
            })?;

        Ok(OrderApprovalDto::from(&approval))
    }
}
