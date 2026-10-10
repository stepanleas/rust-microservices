use crate::dto::responses::OrderApprovalResponse;
use application::dtos::OrderApprovalDto;
use chrono::Utc;
use domain::enums::OrderApprovalStatus;
use uuid::Uuid;

fn status_json(status: OrderApprovalStatus) -> serde_json::Value {
    let response = OrderApprovalResponse::from(OrderApprovalDto {
        id: Uuid::new_v4(),
        saga_id: Uuid::new_v4(),
        order_id: Uuid::new_v4(),
        status,
        failure_messages: vec![],
        created_at: Utc::now(),
    });

    serde_json::to_value(response).unwrap()["status"].clone()
}

#[test]
fn test_order_approval_status_is_serialized_as_upper_case_string() {
    assert_eq!("APPROVED", status_json(OrderApprovalStatus::Approved));
    assert_eq!("REJECTED", status_json(OrderApprovalStatus::Rejected));
    assert_eq!("CANCELLED", status_json(OrderApprovalStatus::Cancelled));
}
