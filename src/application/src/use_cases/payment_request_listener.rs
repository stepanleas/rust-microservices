use crate::commands::ProcessPaymentCommand;
use crate::ports::r#in::messages::PaymentRequestMessageListener;
use crate::use_cases::ProcessPayment;
use shared::error::AppResult;
use std::sync::Arc;

pub struct ApplicationPaymentRequestMessageListener {
    process_payment: Arc<ProcessPayment>,
}

impl ApplicationPaymentRequestMessageListener {
    pub fn new(process_payment: Arc<ProcessPayment>) -> Self {
        Self { process_payment }
    }
}

impl PaymentRequestMessageListener for ApplicationPaymentRequestMessageListener {
    fn payment_requested(&self, command: ProcessPaymentCommand) -> AppResult<()> {
        let order_id = command.order_id();

        match self.process_payment.execute(command) {
            Ok(Some(payment)) => {
                tracing::info!(
                    "Payment {} processed for order {} with status {}",
                    payment.id(),
                    order_id,
                    payment.payment_status_as_str(),
                );
                Ok(())
            }
            Ok(None) => Ok(()),
            Err(error) => {
                tracing::error!(
                    "Error while processing payment for order {}: {}",
                    order_id,
                    error
                );
                Err(error)
            }
        }
    }
}
