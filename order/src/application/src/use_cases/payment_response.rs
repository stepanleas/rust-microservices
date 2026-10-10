use crate::ports::r#in::messages::PaymentResponseMessageListener;
use crate::saga::dtos::PaymentResponse;
use crate::saga::payment_saga::PaymentSaga;
use shared::error::AppResult;

/// Finishes the order saga with the payment's reply. Idempotent: a reply for a saga that
/// already moved on is ignored.
pub struct HandlePaymentResponse {
    saga: PaymentSaga,
}

impl HandlePaymentResponse {
    pub fn new(saga: PaymentSaga) -> Self {
        Self { saga }
    }
}

impl PaymentResponseMessageListener for HandlePaymentResponse {
    fn payment_completed(&self, response: PaymentResponse) -> AppResult<()> {
        self.saga.process(&response)
    }

    fn payment_failed(&self, response: PaymentResponse) -> AppResult<()> {
        self.saga.rollback(&response)
    }
}
