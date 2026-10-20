mod common;

use application::outbox::{RestaurantApprovalResponsePayload, restaurant_approval_response};
use application::ports::out::repositories::{
    OrderApprovalRepository, ProductRepository, StockChange,
};
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

fn change(product_id: ProductId, quantity: i32) -> StockChange {
    StockChange {
        product_id,
        quantity,
    }
}

/// Stores an approved order that took `taken` items of a product that had 10.
fn approve(pool: &DbPool, taken: i32) -> anyhow::Result<(OrderApproval, ProductId)> {
    let product_repository = PostgresProductRepository::new(pool);
    let product_id = ProductId::new();
    product_repository.save(&Product::new(product_id, "Pizza".into(), 10, dec!(12.50)))?;

    let approval = OrderApproval::approved(Uuid::new_v4(), OrderId::new());
    PostgresOrderApprovalRepository::new(pool).save_with_outbox(
        &approval,
        &[change(product_id, -taken)],
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

    approval_repository.save_with_outbox(&approval, &[change(product_id, -4)], &message)?;

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
        &[change(product_id, -9)],
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

    approval.cancel()?;

    approval_repository.save_cancellation(&approval, &[change(product_id, 4)])?;

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

    approval_repository.save_cancellation(&approval, &[change(product_id, 4)])?;

    // A duplicate cancellation (e.g. computed from a stale read) must change nothing.
    let result = approval_repository.save_cancellation(&approval, &[change(product_id, 4)]);

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

#[tokio::test]
async fn test_concurrent_approvals_of_one_product_each_take_their_stock() -> anyhow::Result<()> {
    let (_container, pool) = common::start_postgres().await?;
    let approval_repository = PostgresOrderApprovalRepository::new(&pool);
    let product_id = ProductId::new();
    PostgresProductRepository::new(&pool).save(&Product::new(
        product_id,
        "Pizza".into(),
        10,
        dec!(12.50),
    ))?;

    // Two replicas read a stock of 10 at the same time, then take 3 and 4: neither may undo the
    // other's decrease (it did when the approval wrote back the absolute quantity it computed).
    for taken in [3, 4] {
        let approval = OrderApproval::approved(Uuid::new_v4(), OrderId::new());
        approval_repository.save_with_outbox(
            &approval,
            &[change(product_id, -taken)],
            &restaurant_approval_response(&approval)?,
        )?;
    }

    assert_eq!(3, quantity(&pool, product_id)?);

    Ok(())
}

#[tokio::test]
async fn test_save_approval_rolls_back_when_the_stock_was_taken_meanwhile() -> anyhow::Result<()> {
    let (_container, pool) = common::start_postgres().await?;
    let approval_repository = PostgresOrderApprovalRepository::new(&pool);
    let outbox_repository = PostgresOutboxRepository::new(&pool);
    let (_, product_id) = approve(&pool, 8)?;

    // Computed from a stale read of 10: only 2 are left.
    let late = OrderApproval::approved(Uuid::new_v4(), OrderId::new());
    let result = approval_repository.save_with_outbox(
        &late,
        &[change(product_id, -5)],
        &restaurant_approval_response(&late)?,
    );

    assert!(result.is_err());
    assert_eq!(2, quantity(&pool, product_id)?);
    assert!(
        approval_repository
            .find_by_order_id(late.order_id())?
            .is_none()
    );
    assert_eq!(
        1,
        outbox_repository
            .find_by_status(OutboxStatus::Started, 10)?
            .len()
    );

    Ok(())
}
