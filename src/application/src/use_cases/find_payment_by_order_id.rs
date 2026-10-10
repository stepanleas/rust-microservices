use crate::dtos::PaymentDto;
use crate::ports::out::repositories::PaymentRepository;
use crate::queries::FindPaymentByOrderIdQuery;
use shared::domain::value_objects::OrderId;
use shared::error::{AppError, AppResult};
use std::sync::Arc;

pub struct FindPaymentByOrderId {
    repository: Arc<dyn PaymentRepository>,
}

impl FindPaymentByOrderId {
    pub fn new(repository: Arc<dyn PaymentRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, query: FindPaymentByOrderIdQuery) -> AppResult<PaymentDto> {
        self.repository
            .find_by_order_id(OrderId::from_uuid(query.order_id()))?
            .map(PaymentDto::from)
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "Could not find payment for order with id: {}",
                    query.order_id()
                ))
            })
    }
}
