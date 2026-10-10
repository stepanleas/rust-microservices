mod common;

use application::outbox::{RestaurantOutboxMessage, restaurant_approval_response};
use application::ports::out::repositories::OrderApprovalRepository;
use domain::aggregates::order_approval::OrderApproval;
use infrastructure::DbPool;
use infrastructure::postgres::{PostgresOrderApprovalRepository, PostgresOutboxRepository};
use shared::domain::value_objects::OrderId;
use shared::outbox::OutboxStatus;
use shared::outbox::ports::OutboxRepository;
use uuid::Uuid;

fn store_message(pool: &DbPool) -> anyhow::Result<RestaurantOutboxMessage> {
    let approval = OrderApproval::approved(Uuid::new_v4(), OrderId::new());
    let message = restaurant_approval_response(&approval)?;
    PostgresOrderApprovalRepository::new(pool).save_with_outbox(&approval, &[], &message)?;

    Ok(message)
}

#[tokio::test]
async fn test_find_by_status_orders_and_limits() -> anyhow::Result<()> {
    let (_container, pool) = common::start_postgres().await?;
    let repository = PostgresOutboxRepository::new(&pool);

    let first = store_message(&pool)?;
    let second = store_message(&pool)?;
    let _third = store_message(&pool)?;

    let messages = repository.find_by_status(OutboxStatus::Started, 2)?;

    assert_eq!(2, messages.len());
    assert_eq!(first.id(), messages[0].id());
    assert_eq!(second.id(), messages[1].id());

    Ok(())
}

#[tokio::test]
async fn test_mark_completed() -> anyhow::Result<()> {
    let (_container, pool) = common::start_postgres().await?;
    let repository = PostgresOutboxRepository::new(&pool);
    let message = store_message(&pool)?;

    repository.mark_completed(message.id())?;

    assert!(
        repository
            .find_by_status(OutboxStatus::Started, 10)?
            .is_empty()
    );
    let completed = repository.find_by_status(OutboxStatus::Completed, 10)?;
    assert_eq!(1, completed.len());
    assert_eq!(message.id(), completed[0].id());

    Ok(())
}

#[tokio::test]
async fn test_mark_attempt_failed_until_max_attempts() -> anyhow::Result<()> {
    let (_container, pool) = common::start_postgres().await?;
    let repository = PostgresOutboxRepository::new(&pool);
    let message = store_message(&pool)?;

    assert_eq!(
        OutboxStatus::Started,
        repository.mark_attempt_failed(message.id(), 2)?
    );

    let pending = repository.find_by_status(OutboxStatus::Started, 10)?;
    assert_eq!(1, pending.len());
    assert_eq!(1, pending[0].attempts());

    assert_eq!(
        OutboxStatus::Failed,
        repository.mark_attempt_failed(message.id(), 2)?
    );

    assert!(
        repository
            .find_by_status(OutboxStatus::Started, 10)?
            .is_empty()
    );
    let failed = repository.find_by_status(OutboxStatus::Failed, 10)?;
    assert_eq!(1, failed.len());
    assert_eq!(2, failed[0].attempts());

    Ok(())
}
