use application::use_cases::{GetOrderApproval, GetProduct, ListProducts};
use std::sync::Arc;

/// What the controllers need: the use cases (built in the starter's wiring) and the
/// environment name for the info endpoint.
#[derive(Clone)]
pub struct AppState {
    pub environment: String,
    pub get_order_approval: Arc<GetOrderApproval>,
    pub get_product: Arc<GetProduct>,
    pub list_products: Arc<ListProducts>,
}
