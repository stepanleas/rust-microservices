use crate::commands::{ApproveOrderCommand, CancelOrderApprovalCommand};
use crate::dtos::ProductDto;
use shared::error::AppResult;
use uuid::Uuid;

/// Product replication from the catalog service (`product-created/updated/deleted`).
#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait ProductMessageListener: Send + Sync {
    fn product_created(&self, product: ProductDto) -> AppResult<()>;
    fn product_updated(&self, product: ProductDto) -> AppResult<()>;
    fn product_deleted(&self, product_id: Uuid) -> AppResult<()>;
}

/// The restaurant's part of the order saga. Messages are delivered at least once, so every
/// method must be idempotent.
#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
pub trait RestaurantApprovalMessageListener: Send + Sync {
    /// `restaurant-approval-request`: approve or reject an order.
    fn approval_requested(&self, command: ApproveOrderCommand) -> AppResult<()>;

    /// `restaurant-approval-cancel-request`: compensation for an approved order.
    fn approval_cancel_requested(&self, command: CancelOrderApprovalCommand) -> AppResult<()>;
}
