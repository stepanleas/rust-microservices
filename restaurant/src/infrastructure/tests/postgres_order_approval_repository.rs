mod common;

use application::outbox::{RestaurantApprovalResponsePayload, restaurant_approval_response};
use application::ports::out::repositories::{OrderApprovalRepository, ProductRepository};
use domain::aggregates::order_approval::OrderApproval;
use domain::aggregates::product::Product;
use domain::enums::OrderApprovalStatus;
use infrastructure::DbPool;
use infrastructure::postgres::{
    PostgresOrderApprovalRepository, PostgresOutboxRepository, PostgresProductRepository,
};
use rust_decimal::dec;
use shared::domain::value_objects::{OrderId, ProductId};
use shared::outbox::OutboxStatus;
use shared::outbox::ports::OutboxRepository;
use uuid::Uuid;

fn quantity(pool: &DbPool, product_id: ProductId) -> anyhow::Result<i32> {
    Ok(PostgresProductRepository::new(pool)
        .find_by_id(product_id)?
        .expect("product must exist")
        .quantity())
}

/// Stores an approved order that took `taken` items of a product that had 10.
fn approve(pool: &DbPool, taken: i32) -> anyhow::Result<(OrderApproval, ProductId)> {
    let product_repository = PostgresProductRepository::new(pool);
    let product_id = ProductId::new();
    product_repository.save(&Product::new(product_id, "Pizza".into(), 10, dec!(12.50)))?;

    let approval = OrderApproval::approved(Uuid::new_v4(), OrderId::new());
    let mut product = product_repository.find_by_id(product_id)?.unwrap();
    product.decrease_stock(taken)?;
    PostgresOrderApprovalRepository::new(pool).save_with_outbox(
        &approval,
        &[product],
        &restaurant_approval_response(&approval)?,
    )?;

    Ok((approval, product_id))
}

#[tokio::test]
async fn test_save_approval_with_stock_and_outbox_in_one_transaction() -> anyhow::Result<()> {
    let (_container, pool) = common::start_postgres().await?;
    let product_repository = PostgresProductRepository::new(&pool);
    let approval_repository = PostgresOrderApprovalRepository::new(&pool);
    let outbox_repository = PostgresOutboxRepository::new(&pool);

    let product_id = ProductId::new();
    product_repository.save(&Product::new(product_id, "Pizza".into(), 10, dec!(12.50)))?;

    let saga_id = Uuid::new_v4();
    let order_id = OrderId::new();
    let approval = OrderApproval::approved(saga_id, order_id);
    let message = restaurant_approval_response(&approval)?;
    let mut product = product_repository.find_by_id(product_id)?.unwrap();
    product.decrease_stock(4)?;

    approval_repository.save_with_outbox(&approval, &[product], &message)?;

    let saved = approval_repository
        .find_by_order_id(order_id)?
        .expect("approval must be stored");
    assert_eq!(approval.id(), saved.id());
    assert_eq!(saga_id, saved.saga_id());
    assert_eq!(OrderApprovalStatus::Approved, saved.status());
    assert!(saved.failure_messages().is_empty());

    assert_eq!(6, quantity(&pool, product_id)?);

    let pending = outbox_repository.find_by_status(OutboxStatus::Started, 10)?;
    assert_eq!(1, pending.len());
    assert_eq!(message.id(), pending[0].id());
    let payload: RestaurantApprovalResponsePayload = pending[0].payload_as()?;
    assert_eq!(*order_id.as_uuid(), payload.order_id);
    assert_eq!("APPROVED", payload.order_approval_status);

    Ok(())
}

#[tokio::test]
async fn test_save_rejected_approval_keeps_failure_messages() -> anyhow::Result<()> {
    let (_container, pool) = common::start_postgres().await?;
    let approval_repository = PostgresOrderApprovalRepository::new(&pool);

    let order_id = OrderId::new();
    let approval = OrderApproval::rejected(
        Uuid::new_v4(),
        order_id,
        vec!["first reason".into(), "second reason".into()],
    );
    let message = restaurant_approval_response(&approval)?;

    approval_repository.save_with_outbox(&approval, &[], &message)?;

    let saved = approval_repository.find_by_order_id(order_id)?.unwrap();
    assert_eq!(OrderApprovalStatus::Rejected, saved.status());
    assert_eq!(
        vec!["first reason".to_string(), "second reason".to_string()],
        saved.failure_messages(),
    );
    assert!(
        approval_repository
            .find_by_order_id(OrderId::new())?
            .is_none()
    );

    Ok(())
}

#[tokio::test]
async fn test_save_approval_rolls_back_when_order_already_processed() -> anyhow::Result<()> {
    let (_container, pool) = common::start_postgres().await?;
    let product_repository = PostgresProductRepository::new(&pool);
    let approval_repository = PostgresOrderApprovalRepository::new(&pool);
    let outbox_repository = PostgresOutboxRepository::new(&pool);

    let product_id = ProductId::new();
    product_repository.save(&Product::new(product_id, "Pizza".into(), 10, dec!(12.50)))?;

    let order_id = OrderId::new();
    let first = OrderApproval::approved(Uuid::new_v4(), order_id);
    approval_repository.save_with_outbox(&first, &[], &restaurant_approval_response(&first)?)?;

    // Same order again: unique(order_id) must abort the whole transaction.
    let duplicate = OrderApproval::approved(Uuid::new_v4(), order_id);
    let result = approval_repository.save_with_outbox(
        &duplicate,
        &[Product::new(product_id, "Pizza".into(), 1, dec!(12.50))],
        &restaurant_approval_response(&duplicate)?,
    );

    assert!(result.is_err());
    assert_eq!(10, quantity(&pool, product_id)?);
    assert_eq!(
        1,
        outbox_repository
            .find_by_status(OutboxStatus::Started, 10)?
            .len(),
    );

    Ok(())
}

#[tokio::test]
async fn test_save_cancellation_restocks_and_cancels_in_one_transaction() -> anyhow::Result<()> {
    let (_container, pool) = common::start_postgres().await?;
    let approval_repository = PostgresOrderApprovalRepository::new(&pool);
    let outbox_repository = PostgresOutboxRepository::new(&pool);
    let (mut approval, product_id) = approve(&pool, 4)?;
    assert_eq!(6, quantity(&pool, product_id)?);

    let mut product = PostgresProductRepository::new(&pool)
        .find_by_id(product_id)?
        .unwrap();
    product.increase_stock(4)?;
    approval.cancel()?;

    approval_repository.save_cancellation(&approval, &[product])?;

    let saved = approval_repository
        .find_by_order_id(approval.order_id())?
        .unwrap();
    assert_eq!(OrderApprovalStatus::Cancelled, saved.status());
    assert_eq!(10, quantity(&pool, product_id)?);
    // No answer is sent for a compensation: only the approval response is in the outbox.
    assert_eq!(
        1,
        outbox_repository
            .find_by_status(OutboxStatus::Started, 10)?
            .len()
    );

    Ok(())
}

#[tokio::test]
async fn test_save_cancellation_rolls_back_when_approval_is_no_longer_approved()
-> anyhow::Result<()> {
    let (_container, pool) = common::start_postgres().await?;
    let approval_repository = PostgresOrderApprovalRepository::new(&pool);
    let (mut approval, product_id) = approve(&pool, 4)?;
    approval.cancel()?;

    let restocked = |quantity| Product::new(product_id, "Pizza".into(), quantity, dec!(12.50));
    approval_repository.save_cancellation(&approval, &[restocked(10)])?;

    // A duplicate cancellation (e.g. computed from a stale read) must change nothing.
    let result = approval_repository.save_cancellation(&approval, &[restocked(14)]);

    assert!(result.is_err());
    assert_eq!(10, quantity(&pool, product_id)?);
    assert_eq!(
        OrderApprovalStatus::Cancelled,
        approval_repository
            .find_by_order_id(approval.order_id())?
            .unwrap()
            .status()
    );

    Ok(())
}
