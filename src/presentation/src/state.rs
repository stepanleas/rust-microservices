use application::use_cases::{CreateCustomer, UpdateCustomer};
use std::sync::Arc;

/// Everything the controllers need: the use cases, built in `starter::wiring`.
#[derive(Clone)]
pub struct AppState {
    pub environment: String,

    pub create_customer: Arc<CreateCustomer>,
    pub update_customer: Arc<UpdateCustomer>,
}
