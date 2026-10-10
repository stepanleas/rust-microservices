use crate::commands::{ApproveOrderCommand, CancelOrderApprovalCommand};
use crate::ports::r#in::messages::RestaurantApprovalMessageListener;
use crate::use_cases::{ApproveOrder, CancelOrderApproval};
use shared::error::AppResult;
use std::sync::Arc;

pub struct ApplicationRestaurantApprovalMessageListener {
    approve_order: Arc<ApproveOrder>,
    cancel_order_approval: Arc<CancelOrderApproval>,
}

impl ApplicationRestaurantApprovalMessageListener {
    pub fn new(
        approve_order: Arc<ApproveOrder>,
        cancel_order_approval: Arc<CancelOrderApproval>,
    ) -> Self {
        Self {
            approve_order,
            cancel_order_approval,
        }
    }
}

impl RestaurantApprovalMessageListener for ApplicationRestaurantApprovalMessageListener {
    fn approval_requested(&self, command: ApproveOrderCommand) -> AppResult<()> {
        self.approve_order.execute(command).map(|_| ())
    }

    fn approval_cancel_requested(&self, command: CancelOrderApprovalCommand) -> AppResult<()> {
        self.cancel_order_approval.execute(command).map(|_| ())
    }
}
