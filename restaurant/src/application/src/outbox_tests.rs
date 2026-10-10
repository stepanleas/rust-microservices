use crate::outbox::{
    OutboxMessageType, RestaurantApprovalResponsePayload, restaurant_approval_response,
};
use domain::aggregates::order_approval::OrderApproval;
use shared::domain::value_objects::OrderId;
use shared::outbox::{MessageKind, OutboxStatus};
use uuid::Uuid;

#[test]
fn test_create_restaurant_approval_response_message() -> anyhow::Result<()> {
    let saga_id = Uuid::new_v4();
    let order_id = OrderId::new();
    let approval = OrderApproval::rejected(saga_id, order_id, vec!["Out of stock".into()]);

    let message = restaurant_approval_response(&approval)?;

    assert_ne!(Uuid::nil(), message.id());
    assert_eq!(saga_id, message.saga_id());
    assert_eq!(
        OutboxMessageType::RestaurantApprovalResponse,
        message.message_type(),
    );
    assert_eq!(OutboxStatus::Started, message.status());
    assert_eq!(0, message.attempts());

    let payload: RestaurantApprovalResponsePayload = message.payload_as()?;
    assert_eq!(saga_id, payload.saga_id);
    assert_eq!(*order_id.as_uuid(), payload.order_id);
    assert_eq!("REJECTED", payload.order_approval_status);
    assert_eq!(vec!["Out of stock".to_string()], payload.failure_messages);
    assert!(payload.created_at.ends_with('Z'));

    Ok(())
}

#[test]
fn test_outbox_message_type_conversions() -> anyhow::Result<()> {
    assert_eq!(
        OutboxMessageType::RestaurantApprovalResponse,
        OutboxMessageType::parse(OutboxMessageType::RestaurantApprovalResponse.as_str())?,
    );
    assert!(OutboxMessageType::parse("UNKNOWN").is_err());

    Ok(())
}
