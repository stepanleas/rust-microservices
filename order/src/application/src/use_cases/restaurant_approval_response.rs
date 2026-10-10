use crate::ports::r#in::messages::RestaurantApprovalResponseMessageListener;
use crate::saga::dtos::RestaurantApprovalResponse;
use crate::saga::restaurant_approval_saga::RestaurantApprovalSaga;
use shared::error::AppResult;

/// Moves the order saga on with the restaurant's reply. Idempotent: a reply for a saga that
/// already moved on is ignored.
pub struct HandleRestaurantApprovalResponse {
    saga: RestaurantApprovalSaga,
}

impl HandleRestaurantApprovalResponse {
    pub fn new(saga: RestaurantApprovalSaga) -> Self {
        Self { saga }
    }
}

impl RestaurantApprovalResponseMessageListener for HandleRestaurantApprovalResponse {
    fn order_approved(&self, response: RestaurantApprovalResponse) -> AppResult<()> {
        self.saga.process(&response)
    }

    fn order_rejected(&self, response: RestaurantApprovalResponse) -> AppResult<()> {
        self.saga.rollback(&response)
    }
}
