use crate::commands::{CancelOrderApprovalCommand, OrderProductDto};
use crate::ports::out::repositories::{MockOrderApprovalRepository, MockProductRepository};
use crate::use_cases::CancelOrderApproval;
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
    order_id: OrderId,
    lines: &[(ProductId, i32)],
) -> CancelOrderApprovalCommand {
    CancelOrderApprovalCommand::new(
        saga_id,
        *order_id.as_uuid(),
        lines
            .iter()
            .map(|(product_id, quantity)| OrderProductDto::new(*product_id.as_uuid(), *quantity))
            .collect(),
    )
}

fn use_case(
    approval_repository: MockOrderApprovalRepository,
    product_repository: MockProductRepository,
) -> CancelOrderApproval {
    CancelOrderApproval::new(Arc::new(approval_repository), Arc::new(product_repository))
}

#[test]
fn test_cancel_approved_order_restocks_and_cancels() -> anyhow::Result<()> {
    let saga_id = Uuid::new_v4();
    let order_id = OrderId::new();
    let pizza = ProductId::new();
    let pasta = ProductId::new();
    let deleted = ProductId::new();
    let approval = OrderApproval::approved(saga_id, order_id);
    let approval_id = approval.id();

    let mut approval_repository = MockOrderApprovalRepository::new();
    let mut product_repository = MockProductRepository::new();

    approval_repository
        .expect_find_by_order_id()
        .once()
        .with(predicate::eq(order_id))
        .returning(move |_| Ok(Some(approval.clone())));
    product_repository
        .expect_find_by_ids()
        .once()
        .with(predicate::eq(vec![pizza, pasta, deleted]))
        .returning(move |_| {
            // `deleted` no longer exists.
            Ok(vec![
                Product::new(pizza, "Pizza".into(), 1, dec!(10.00)),
                Product::new(pasta, "Pasta".into(), 0, dec!(8.00)),
            ])
        });
    approval_repository
        .expect_save_cancellation()
        .once()
        .withf(move |approval, products| {
            approval.id() == approval_id
                && approval.status() == OrderApprovalStatus::Cancelled
                && products.len() == 2
                && products[0].id() == pizza
                && products[0].quantity() == 1 + 2 + 1
                && products[1].id() == pasta
                && products[1].quantity() == 3
        })
        .returning(|_, _| Ok(()));

    let cancelled = use_case(approval_repository, product_repository)
        .execute(command(
            saga_id,
            order_id,
            &[(pizza, 2), (pasta, 3), (deleted, 5), (pizza, 1)],
        ))?
        .expect("approval must be cancelled");

    assert_eq!(OrderApprovalStatus::Cancelled, cancelled.status());

    Ok(())
}

#[test]
fn test_cancel_rejected_order_is_a_no_op() -> anyhow::Result<()> {
    let order_id = OrderId::new();
    let approval = OrderApproval::rejected(Uuid::new_v4(), order_id, vec!["Out of stock".into()]);

    let mut approval_repository = MockOrderApprovalRepository::new();
    let mut product_repository = MockProductRepository::new();

    approval_repository
        .expect_find_by_order_id()
        .once()
        .returning(move |_| Ok(Some(approval.clone())));
    product_repository.expect_find_by_ids().never();
    approval_repository.expect_save_cancellation().never();

    let result = use_case(approval_repository, product_repository).execute(command(
        Uuid::new_v4(),
        order_id,
        &[(ProductId::new(), 1)],
    ))?;

    assert!(result.is_none());

    Ok(())
}

#[test]
fn test_cancel_unknown_order_is_a_no_op() -> anyhow::Result<()> {
    let mut approval_repository = MockOrderApprovalRepository::new();
    let mut product_repository = MockProductRepository::new();

    approval_repository
        .expect_find_by_order_id()
        .once()
        .returning(|_| Ok(None));
    product_repository.expect_find_by_ids().never();
    approval_repository.expect_save_cancellation().never();

    let result = use_case(approval_repository, product_repository).execute(command(
        Uuid::new_v4(),
        OrderId::new(),
        &[(ProductId::new(), 1)],
    ))?;

    assert!(result.is_none());

    Ok(())
}

#[test]
fn test_repeated_cancel_is_a_no_op() -> anyhow::Result<()> {
    let saga_id = Uuid::new_v4();
    let order_id = OrderId::new();
    let mut approval = OrderApproval::approved(saga_id, order_id);
    approval.cancel()?;

    let mut approval_repository = MockOrderApprovalRepository::new();
    let mut product_repository = MockProductRepository::new();

    approval_repository
        .expect_find_by_order_id()
        .once()
        .returning(move |_| Ok(Some(approval.clone())));
    product_repository.expect_find_by_ids().never();
    approval_repository.expect_save_cancellation().never();

    let result = use_case(approval_repository, product_repository).execute(command(
        saga_id,
        order_id,
        &[(ProductId::new(), 1)],
    ))?;

    assert!(result.is_none());

    Ok(())
}

#[test]
fn test_cancel_propagates_save_error() {
    let order_id = OrderId::new();
    let approval = OrderApproval::approved(Uuid::new_v4(), order_id);

    let mut approval_repository = MockOrderApprovalRepository::new();
    let mut product_repository = MockProductRepository::new();

    approval_repository
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(approval.clone())));
    product_repository
        .expect_find_by_ids()
        .returning(|_| Ok(vec![]));
    approval_repository
        .expect_save_cancellation()
        .returning(|_, _| Err(anyhow::anyhow!("database unavailable")));

    let error = use_case(approval_repository, product_repository)
        .execute(command(Uuid::new_v4(), order_id, &[(ProductId::new(), 1)]))
        .unwrap_err();

    assert!(matches!(error, AppError::Internal(_)));
}

#[test]
fn test_cancel_with_invalid_quantity_fails_without_saving() {
    let order_id = OrderId::new();
    let product_id = ProductId::new();
    let approval = OrderApproval::approved(Uuid::new_v4(), order_id);

    let mut approval_repository = MockOrderApprovalRepository::new();
    let mut product_repository = MockProductRepository::new();

    approval_repository
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(approval.clone())));
    product_repository.expect_find_by_ids().returning(move |_| {
        Ok(vec![Product::new(
            product_id,
            "Pizza".into(),
            1,
            dec!(10.00),
        )])
    });
    approval_repository.expect_save_cancellation().never();

    let error = use_case(approval_repository, product_repository)
        .execute(command(Uuid::new_v4(), order_id, &[(product_id, 0)]))
        .unwrap_err();

    assert!(matches!(error, AppError::BusinessRule(_)));
}
