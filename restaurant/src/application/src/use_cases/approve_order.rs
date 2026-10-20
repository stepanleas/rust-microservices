use crate::commands::ApproveOrderCommand;
use crate::outbox::restaurant_approval_response;
use crate::ports::out::repositories::{OrderApprovalRepository, ProductRepository};
use domain::aggregates::order_approval::OrderApproval;
use domain::services::{RequestedProduct, RestaurantApprovalDomainService};
use shared::domain::value_objects::{OrderId, ProductId};
use shared::error::AppResult;
use std::sync::Arc;

pub struct ApproveOrder {
    approval_repository: Arc<dyn OrderApprovalRepository>,
    product_repository: Arc<dyn ProductRepository>,
}

impl ApproveOrder {
    pub fn new(
        approval_repository: Arc<dyn OrderApprovalRepository>,
        product_repository: Arc<dyn ProductRepository>,
    ) -> Self {
        Self {
            approval_repository,
            product_repository,
        }
    }

    /// Approves or rejects the order and stores the response in the outbox.
    ///
    /// Idempotent: when the order was already processed (e.g. the request was
    /// redelivered), the stored approval is returned and nothing is written.
    pub fn execute(&self, command: ApproveOrderCommand) -> AppResult<OrderApproval> {
        let order_id = OrderId::from_uuid(command.order_id());

        if let Some(existing) = self.approval_repository.find_by_order_id(order_id)? {
            tracing::info!(
                "Order {} was already processed with status {}, skipping",
                command.order_id(),
                existing.status(),
            );

            return Ok(existing);
        }

        let requested: Vec<RequestedProduct> = command
            .products()
            .iter()
            .map(|item| {
                RequestedProduct::new(ProductId::from_uuid(item.product_id()), item.quantity())
            })
            .collect();

        let mut products = self.product_repository.find_by_ids(
            requested
                .iter()
                .map(|item| item.product_id())
                .collect::<Vec<_>>(),
        )?;

        let approval = RestaurantApprovalDomainService::approve_order(
            command.saga_id(),
            order_id,
            &requested,
            &mut products,
        );

        let updated_products = if approval.is_approved() {
            products
        } else {
            vec![]
        };

        let message = restaurant_approval_response(&approval)?;
        self.approval_repository
            .save_with_outbox(&approval, &updated_products, &message)?;
        let decision = if approval.is_approved() {
            "approved"
        } else {
            "rejected"
        };
        metrics::counter!("restaurant_order_approvals_total", "decision" => decision).increment(1);

        tracing::info!(
            "Order {} {} by restaurant (saga {})",
            command.order_id(),
            approval.status(),
            command.saga_id(),
        );

        Ok(approval)
    }
}
