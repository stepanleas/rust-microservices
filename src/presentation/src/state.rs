use application::use_cases::{CreateOrder, TrackOrder};
use std::sync::Arc;

/// What the controllers need: the use cases (wired in the starter) and the environment name
/// shown by the info endpoint.
#[derive(Clone)]
pub struct AppState {
    pub environment: String,
    pub create_order: Arc<CreateOrder>,
    pub track_order: Arc<TrackOrder>,
}
