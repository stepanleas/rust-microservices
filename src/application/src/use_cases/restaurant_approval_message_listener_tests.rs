use crate::commands::{ApproveOrderCommand, CancelOrderApprovalCommand, OrderProductDto};
use crate::ports::r#in::messages::RestaurantApprovalMessageListener;
use crate::ports::out::repositories::{MockOrderApprovalRepository, MockProductRepository};
use crate::use_cases::{
    ApplicationRestaurantApprovalMessageListener, ApproveOrder, CancelOrderApproval,
};
use rust_decimal::dec;
use std::sync::Arc;
use uuid::Uuid;

fn listener(
    approval_repository: MockOrderApprovalRepository,
    product_repository: MockProductRepository,
) -> ApplicationRestaurantApprovalMessageListener {
    let approval_repository = Arc::new(approval_repository);
    let product_repository = Arc::new(product_repository);

    ApplicationRestaurantApprovalMessageListener::new(
        Arc::new(ApproveOrder::new(
            approval_repository.clone(),
            product_repository.clone(),
        )),
        Arc::new(CancelOrderApproval::new(
            approval_repository,
            product_repository,
        )),
    )
}

#[test]
fn test_approval_requested_executes_approve_order() -> anyhow::Result<()> {
    let mut approval_repository = MockOrderApprovalRepository::new();
    let mut product_repository = MockProductRepository::new();

    approval_repository
        .expect_find_by_order_id()
        .once()
        .returning(|_| Ok(None));
    product_repository
        .expect_find_by_ids()
        .once()
        .returning(|_| Ok(vec![]));
    approval_repository
        .expect_save_with_outbox()
        .once()
        .returning(|_, _, _| Ok(()));

    listener(approval_repository, product_repository).approval_requested(
        ApproveOrderCommand::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            dec!(10.00),
            vec![OrderProductDto::new(Uuid::new_v4(), 1)],
        ),
    )?;

    Ok(())
}

#[test]
fn test_approval_cancel_requested_executes_cancel_order_approval() -> anyhow::Result<()> {
    let mut approval_repository = MockOrderApprovalRepository::new();
    let product_repository = MockProductRepository::new();

    approval_repository
        .expect_find_by_order_id()
        .once()
        .returning(|_| Ok(None));
    approval_repository.expect_save_cancellation().never();

    listener(approval_repository, product_repository).approval_cancel_requested(
        CancelOrderApprovalCommand::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            vec![OrderProductDto::new(Uuid::new_v4(), 1)],
        ),
    )?;

    Ok(())
}
