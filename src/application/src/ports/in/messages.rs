use crate::commands::ProcessPaymentCommand;
use shared::error::AppResult;

/// Called by the messaging adapter for every `payment-request` message.
#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait PaymentRequestMessageListener: Send + Sync {
    fn payment_requested(&self, command: ProcessPaymentCommand) -> AppResult<()>;
}
