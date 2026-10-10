use crate::commands::{ApproveOrderCommand, OrderProductDto};
use crate::outbox::{OutboxMessageType, RestaurantApprovalResponsePayload};
use crate::ports::out::repositories::{MockOrderApprovalRepository, MockProductRepository};
use crate::use_cases::ApproveOrder;
use domain::aggregates::order_approval::OrderApproval;
use domain::aggregates::product::Product;
use domain::enums::OrderApprovalStatus;
use mockall::predicate;
use rust_decimal::dec;
use shared::domain::value_objects::{OrderId, ProductId};
use shared::error::AppError;
use std::sync::Arc;
use uuid::Uuid;

fn command(
    saga_id: Uuid,
    order_id: Uuid,
    product_id: ProductId,
    quantity: i32,
) -> ApproveOrderCommand {
    ApproveOrderCommand::new(
        saga_id,
        order_id,
        Uuid::new_v4(),
        dec!(30.00),
        vec![OrderProductDto::new(*product_id.as_uuid(), quantity)],
    )
}

#[test]
fn test_approve_order_approves_order() -> anyhow::Result<()> {
    let saga_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let product_id = ProductId::new();

    let mut approval_repository = MockOrderApprovalRepository::new();
    let mut product_repository = MockProductRepository::new();

    approval_repository
        .expect_find_by_order_id()
        .once()
        .with(predicate::eq(OrderId::from_uuid(order_id)))
        .returning(|_| Ok(None));
    product_repository
        .expect_find_by_ids()
        .once()
        .with(predicate::eq(vec![product_id]))
        .returning(move |_| {
            Ok(vec![Product::new(
                product_id,
                "Pizza".into(),
                10,
                dec!(10.00),
            )])
        });
    approval_repository
        .expect_save_with_outbox()
        .once()
        .withf(move |approval, products, message| {
            let payload: RestaurantApprovalResponsePayload = message.payload_as().unwrap();

            approval.status() == OrderApprovalStatus::Approved
                && approval.saga_id() == saga_id
                && products.len() == 1
                && products[0].quantity() == 7
                && message.saga_id() == saga_id
                && message.message_type() == OutboxMessageType::RestaurantApprovalResponse
                && payload.order_id == order_id
                && payload.order_approval_status == "APPROVED"
                && payload.failure_messages.is_empty()
        })
        .returning(|_, _, _| Ok(()));

    let use_case = ApproveOrder::new(Arc::new(approval_repository), Arc::new(product_repository));

    let approval = use_case.execute(command(saga_id, order_id, product_id, 3))?;

    assert_eq!(OrderApprovalStatus::Approved, approval.status());
    assert_eq!(order_id, *approval.order_id().as_uuid());

    Ok(())
}

#[test]
fn test_approve_order_rejects_order() -> anyhow::Result<()> {
    let saga_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let product_id = ProductId::new();

    let mut approval_repository = MockOrderApprovalRepository::new();
    let mut product_repository = MockProductRepository::new();

    approval_repository
        .expect_find_by_order_id()
        .once()
        .returning(|_| Ok(None));
    product_repository
        .expect_find_by_ids()
        .once()
        .returning(move |_| {
            Ok(vec![Product::new(
                product_id,
                "Pizza".into(),
                2,
                dec!(10.00),
            )])
        });
    approval_repository
        .expect_save_with_outbox()
        .once()
        .withf(|approval, products, message| {
            let payload: RestaurantApprovalResponsePayload = message.payload_as().unwrap();

            approval.status() == OrderApprovalStatus::Rejected
                && products.is_empty()
                && payload.order_approval_status == "REJECTED"
                && payload.failure_messages
                    == vec!["Insufficient stock for product Pizza: requested 3, available 2"]
        })
        .returning(|_, _, _| Ok(()));

    let use_case = ApproveOrder::new(Arc::new(approval_repository), Arc::new(product_repository));

    let approval = use_case.execute(command(saga_id, order_id, product_id, 3))?;

    assert_eq!(OrderApprovalStatus::Rejected, approval.status());

    Ok(())
}

#[test]
fn test_approve_order_skips_already_processed_order() -> anyhow::Result<()> {
    let saga_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let existing = OrderApproval::approved(saga_id, OrderId::from_uuid(order_id));
    let existing_id = existing.id();

    let mut approval_repository = MockOrderApprovalRepository::new();
    let mut product_repository = MockProductRepository::new();

    approval_repository
        .expect_find_by_order_id()
        .once()
        .returning(move |_| Ok(Some(existing.clone())));
    product_repository.expect_find_by_ids().never();
    approval_repository.expect_save_with_outbox().never();

    let use_case = ApproveOrder::new(Arc::new(approval_repository), Arc::new(product_repository));

    let approval = use_case.execute(command(saga_id, order_id, ProductId::new(), 1))?;

    assert_eq!(existing_id, approval.id());

    Ok(())
}

#[test]
fn test_approve_order_propagates_save_error() {
    let mut approval_repository = MockOrderApprovalRepository::new();
    let mut product_repository = MockProductRepository::new();

    approval_repository
        .expect_find_by_order_id()
        .returning(|_| Ok(None));
    product_repository
        .expect_find_by_ids()
        .returning(|_| Ok(vec![]));
    approval_repository
        .expect_save_with_outbox()
        .returning(|_, _, _| Err(anyhow::anyhow!("database unavailable")));

    let use_case = ApproveOrder::new(Arc::new(approval_repository), Arc::new(product_repository));

    let error = use_case
        .execute(command(Uuid::new_v4(), Uuid::new_v4(), ProductId::new(), 1))
        .unwrap_err();

    assert!(matches!(error, AppError::Internal(_)));
    assert_eq!("database unavailable", error.to_string());
}
