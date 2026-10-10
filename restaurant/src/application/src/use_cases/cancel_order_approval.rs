use crate::commands::CancelOrderApprovalCommand;
use crate::ports::out::repositories::{OrderApprovalRepository, ProductRepository};
use domain::aggregates::order_approval::OrderApproval;
use domain::enums::OrderApprovalStatus;
use shared::domain::value_objects::{OrderId, ProductId};
use shared::error::AppResult;
use std::sync::Arc;

/// Saga compensation: the order was cancelled after the restaurant approved it (e.g. the
/// payment failed), so the reserved stock is given back. Nothing is sent back to the order
/// service.
pub struct CancelOrderApproval {
    approval_repository: Arc<dyn OrderApprovalRepository>,
    product_repository: Arc<dyn ProductRepository>,
}

impl CancelOrderApproval {
    pub fn new(
        approval_repository: Arc<dyn OrderApprovalRepository>,
        product_repository: Arc<dyn ProductRepository>,
    ) -> Self {
        Self {
            approval_repository,
            product_repository,
        }
    }

    /// Restocks the products of an approved order and marks its approval `CANCELLED`, in one
    /// transaction. Returns the cancelled approval, or `None` when there was nothing to do.
    ///
    /// Idempotent (the cancel request is delivered at least once): an unknown order, a
    /// rejected order (no stock was taken) and an already cancelled order are no-ops.
    /// Products that no longer exist are skipped.
    pub fn execute(&self, command: CancelOrderApprovalCommand) -> AppResult<Option<OrderApproval>> {
        let order_id = OrderId::from_uuid(command.order_id());

        let Some(mut approval) = self.approval_repository.find_by_order_id(order_id)? else {
            tracing::warn!(
                "No approval found for order {} (saga {}), nothing to cancel",
                command.order_id(),
                command.saga_id(),
            );
            return Ok(None);
        };

        if approval.status() != OrderApprovalStatus::Approved {
            tracing::info!(
                "Approval of order {} is {}, nothing to cancel (saga {})",
                command.order_id(),
                approval.status(),
                command.saga_id(),
            );
            return Ok(None);
        }

        if approval.saga_id() != command.saga_id() {
            tracing::warn!(
                "Cancel request for order {} carries saga {} but the approval belongs to saga {}",
                command.order_id(),
                command.saga_id(),
                approval.saga_id(),
            );
        }

        let mut product_ids: Vec<ProductId> = vec![];
        for item in command.products() {
            let product_id = ProductId::from_uuid(item.product_id());
            if !product_ids.contains(&product_id) {
                product_ids.push(product_id);
            }
        }

        let mut products = self.product_repository.find_by_ids(product_ids)?;

        for item in command.products() {
            match products
                .iter_mut()
                .find(|product| *product.id().as_uuid() == item.product_id())
            {
                Some(product) => product.increase_stock(item.quantity())?,
                None => tracing::warn!(
                    "Product {} of cancelled order {} no longer exists, skipping restock of {}",
                    item.product_id(),
                    command.order_id(),
                    item.quantity(),
                ),
            }
        }

        approval.cancel()?;
        self.approval_repository
            .save_cancellation(&approval, &products)?;

        tracing::info!(
            "Approval of order {} cancelled and {} product(s) restocked (saga {})",
            command.order_id(),
            products.len(),
            command.saga_id(),
        );

        Ok(Some(approval))
    }
}
