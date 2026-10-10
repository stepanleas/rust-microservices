use crate::ports::out::repositories::MockOrderApprovalRepository;
use crate::use_cases::GetOrderApproval;
use domain::aggregates::order_approval::OrderApproval;
use domain::enums::OrderApprovalStatus;
use shared::domain::value_objects::OrderId;
use shared::error::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn test_get_order_approval() -> anyhow::Result<()> {
    let order_id = Uuid::new_v4();
    let approval = OrderApproval::rejected(
        Uuid::new_v4(),
        OrderId::from_uuid(order_id),
        vec!["Out of stock".into()],
    );

    let mut repository = MockOrderApprovalRepository::new();
    repository
        .expect_find_by_order_id()
        .once()
        .returning(move |_| Ok(Some(approval.clone())));

    let dto = GetOrderApproval::new(Arc::new(repository)).execute(order_id)?;

    assert_eq!(order_id, dto.order_id);
    assert_eq!(OrderApprovalStatus::Rejected, dto.status);
    assert_eq!(vec!["Out of stock".to_string()], dto.failure_messages);

    Ok(())
}

#[test]
fn test_get_order_approval_not_found() {
    let mut repository = MockOrderApprovalRepository::new();
    repository
        .expect_find_by_order_id()
        .once()
        .returning(|_| Ok(None));

    let error = GetOrderApproval::new(Arc::new(repository))
        .execute(Uuid::new_v4())
        .unwrap_err();

    assert!(matches!(error, AppError::NotFound(_)));
}
