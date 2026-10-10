use crate::dtos::PaymentDto;
use crate::ports::out::repositories::PaymentRepository;
use crate::queries::FindPaymentByIdQuery;
use domain::aggregates::payment::PaymentId;
use shared::error::{AppError, AppResult};
use std::sync::Arc;

pub struct FindPaymentById {
    repository: Arc<dyn PaymentRepository>,
}

impl FindPaymentById {
    pub fn new(repository: Arc<dyn PaymentRepository>) -> Self {
        Self { repository }
    }

    pub fn execute(&self, query: FindPaymentByIdQuery) -> AppResult<PaymentDto> {
        let payment_id = PaymentId::from_uuid(query.id());

        self.repository
            .find_by_id(payment_id)?
            .map(PaymentDto::from)
            .ok_or_else(|| {
                AppError::not_found(format!("Could not find payment with id: {payment_id}"))
            })
    }
}
