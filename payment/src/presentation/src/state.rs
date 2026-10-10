use application::use_cases::{CreatePayment, FindPaymentById, FindPaymentByOrderId, UpdatePayment};
use std::sync::Arc;

/// Everything the controllers need: the use cases, wired in the starter crate.
#[derive(Clone)]
pub struct AppState {
    pub environment: String,
    pub create_payment: Arc<CreatePayment>,
    pub update_payment: Arc<UpdatePayment>,
    pub find_payment_by_id: Arc<FindPaymentById>,
    pub find_payment_by_order_id: Arc<FindPaymentByOrderId>,
}
