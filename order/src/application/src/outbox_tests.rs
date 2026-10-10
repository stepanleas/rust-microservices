use crate::outbox::{OrderApprovalCancelEventPayload, OutboxMessageType};
use domain::aggregates::order::{Order, OrderItem};
use rusty_money::{Money, iso};
use shared::domain::value_objects::{OrderId, OrderItemId, ProductId};
use shared::outbox::MessageKind;
use uuid::Uuid;

#[test]
fn test_outbox_message_type_round_trip() -> anyhow::Result<()> {
    for message_type in [
        OutboxMessageType::RestaurantApprovalRequest,
        OutboxMessageType::PaymentRequest,
        OutboxMessageType::RestaurantApprovalCancelRequest,
    ] {
        assert_eq!(
            message_type,
            OutboxMessageType::parse(message_type.as_str())?
        );
    }

    assert_eq!(
        "RESTAURANT_APPROVAL_CANCEL_REQUEST",
        OutboxMessageType::RestaurantApprovalCancelRequest.as_str(),
    );
    assert!(OutboxMessageType::parse("UNKNOWN").is_err());

    Ok(())
}

#[test]
fn test_cancel_payload_carries_the_order_products() -> anyhow::Result<()> {
    let order_id = OrderId::new();
    let product_id = ProductId::new();
    let saga_id = Uuid::new_v4();
    let order = Order::builder()
        .id(order_id)
        .items(vec![OrderItem::new(
            OrderItemId::new(),
            order_id,
            product_id,
            3,
            Money::from_str("10.0", iso::USD)?,
            Money::from_str("30.0", iso::USD)?,
        )])
        .build();

    let payload = OrderApprovalCancelEventPayload::from_order(&order, saga_id);

    assert_eq!(saga_id, payload.saga_id);
    assert_eq!(*order_id.as_uuid(), payload.order_id);
    assert_eq!(1, payload.products.len());
    assert_eq!(*product_id.as_uuid(), payload.products[0].id);
    assert_eq!(3, payload.products[0].quantity);

    Ok(())
}
