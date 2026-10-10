use crate::aggregates::order_approval::OrderApproval;
use crate::enums::OrderApprovalStatus;
use crate::error::DomainError;
use shared::domain::value_objects::OrderId;
use uuid::Uuid;

#[test]
fn test_create_approved_order_approval() {
    let saga_id = Uuid::new_v4();
    let order_id = OrderId::new();

    let approval = OrderApproval::approved(saga_id, order_id);

    assert_ne!(Uuid::nil(), *approval.id().as_uuid());
    assert_eq!(saga_id, approval.saga_id());
    assert_eq!(order_id, approval.order_id());
    assert_eq!(OrderApprovalStatus::Approved, approval.status());
    assert!(approval.is_approved());
    assert!(approval.failure_messages().is_empty());
}

#[test]
fn test_create_rejected_order_approval() {
    let approval =
        OrderApproval::rejected(Uuid::new_v4(), OrderId::new(), vec!["Out of stock".into()]);

    assert_eq!(OrderApprovalStatus::Rejected, approval.status());
    assert!(!approval.is_approved());
    assert_eq!(
        vec!["Out of stock".to_string()],
        approval.failure_messages()
    );
}

#[test]
fn test_order_approval_status_conversion() -> anyhow::Result<()> {
    assert_eq!("APPROVED", OrderApprovalStatus::Approved.as_str());
    assert_eq!("REJECTED", OrderApprovalStatus::Rejected.as_str());
    assert_eq!("CANCELLED", OrderApprovalStatus::Cancelled.as_str());
    assert_eq!(
        OrderApprovalStatus::Approved,
        OrderApprovalStatus::try_from("APPROVED")?,
    );
    assert_eq!(
        OrderApprovalStatus::Rejected,
        OrderApprovalStatus::try_from("REJECTED")?,
    );
    assert_eq!(
        OrderApprovalStatus::Cancelled,
        OrderApprovalStatus::try_from("CANCELLED")?,
    );
    assert!(OrderApprovalStatus::try_from("UNKNOWN").is_err());

    Ok(())
}

#[test]
fn test_cancel_approved_order_approval() -> anyhow::Result<()> {
    let mut approval = OrderApproval::approved(Uuid::new_v4(), OrderId::new());

    approval.cancel()?;

    assert_eq!(OrderApprovalStatus::Cancelled, approval.status());
    assert!(!approval.is_approved());

    Ok(())
}

#[test]
fn test_cancel_rejected_order_approval_fails() {
    let mut approval =
        OrderApproval::rejected(Uuid::new_v4(), OrderId::new(), vec!["Out of stock".into()]);

    let error = approval.cancel().unwrap_err();

    assert!(matches!(
        error,
        DomainError::ApprovalNotCancellable {
            status: OrderApprovalStatus::Rejected,
            ..
        }
    ));
    assert_eq!(OrderApprovalStatus::Rejected, approval.status());
}

#[test]
fn test_cancel_cancelled_order_approval_fails() -> anyhow::Result<()> {
    let mut approval = OrderApproval::approved(Uuid::new_v4(), OrderId::new());
    approval.cancel()?;

    let error = approval.cancel().unwrap_err();

    assert_eq!(
        format!(
            "Order approval of order {} cannot be cancelled in status CANCELLED",
            approval.order_id()
        ),
        error.to_string(),
    );
    assert_eq!(OrderApprovalStatus::Cancelled, approval.status());

    Ok(())
}
