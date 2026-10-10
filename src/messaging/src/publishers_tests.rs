use crate::publishers::encode;
use application::outbox::{
    OrderApprovalCancelEventPayload, OrderApprovalEventPayload, OrderApprovalEventProduct,
    OrderOutboxMessage, OrderPaymentEventPayload, OutboxMessageType,
};
use shared::contracts::AvroMessage;
use shared::contracts::payment::PaymentRequestAvroModel;
use shared::contracts::restaurant::{
    RestaurantApprovalCancelRequestAvroModel, RestaurantApprovalRequestAvroModel,
};
use uuid::Uuid;

#[test]
fn test_encode_restaurant_approval_request() -> anyhow::Result<()> {
    let saga_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let payload = OrderApprovalEventPayload {
        id: Uuid::new_v4(),
        saga_id,
        order_id,
        customer_id: Uuid::new_v4(),
        price: "60.00".into(),
        products: vec![OrderApprovalEventProduct {
            id: product_id,
            quantity: 2,
        }],
        created_at: "2026-10-07T10:00:00+00:00".into(),
    };
    let message = OrderOutboxMessage::new(
        saga_id,
        OutboxMessageType::RestaurantApprovalRequest,
        &payload,
    )?;

    let encoded = encode(&message)?;

    assert_eq!("restaurant-approval-request", encoded.topic);
    assert_eq!(order_id.to_string(), encoded.key);

    let decoded = RestaurantApprovalRequestAvroModel::from_avro_bytes(&encoded.payload)?;
    assert_eq!(1, decoded.len());
    assert_eq!(saga_id.to_string(), decoded[0].saga_id);
    assert_eq!(order_id.to_string(), decoded[0].order_id);
    assert_eq!("60.00", decoded[0].price);
    assert_eq!(product_id.to_string(), decoded[0].products[0].id);
    assert_eq!(2, decoded[0].products[0].quantity);
    assert_eq!("2026-10-07T10:00:00+00:00", decoded[0].created_at);

    Ok(())
}

#[test]
fn test_encode_payment_request() -> anyhow::Result<()> {
    let saga_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let payload = OrderPaymentEventPayload {
        id: Uuid::new_v4(),
        saga_id,
        order_id,
        customer_id,
        price: "60.00".into(),
        created_at: "2026-10-07T10:00:00+00:00".into(),
    };
    let message = OrderOutboxMessage::new(saga_id, OutboxMessageType::PaymentRequest, &payload)?;

    let encoded = encode(&message)?;

    assert_eq!("payment-request", encoded.topic);
    assert_eq!(order_id.to_string(), encoded.key);

    let decoded = PaymentRequestAvroModel::from_avro_bytes(&encoded.payload)?;
    assert_eq!(customer_id.to_string(), decoded[0].customer_id);
    assert_eq!("60.00", decoded[0].price);

    Ok(())
}

#[test]
fn test_encode_restaurant_approval_cancel_request() -> anyhow::Result<()> {
    let saga_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let payload = OrderApprovalCancelEventPayload {
        id: Uuid::new_v4(),
        saga_id,
        order_id,
        products: vec![OrderApprovalEventProduct {
            id: product_id,
            quantity: 3,
        }],
        created_at: "2026-10-07T10:00:00+00:00".into(),
    };
    let message = OrderOutboxMessage::new(
        saga_id,
        OutboxMessageType::RestaurantApprovalCancelRequest,
        &payload,
    )?;

    let encoded = encode(&message)?;

    assert_eq!("restaurant-approval-cancel-request", encoded.topic);
    assert_eq!(order_id.to_string(), encoded.key);

    let decoded = RestaurantApprovalCancelRequestAvroModel::from_avro_bytes(&encoded.payload)?;
    assert_eq!(1, decoded.len());
    assert_eq!(payload.id.to_string(), decoded[0].id);
    assert_eq!(saga_id.to_string(), decoded[0].saga_id);
    assert_eq!(order_id.to_string(), decoded[0].order_id);
    assert_eq!(product_id.to_string(), decoded[0].products[0].id);
    assert_eq!(3, decoded[0].products[0].quantity);
    assert_eq!("2026-10-07T10:00:00+00:00", decoded[0].created_at);

    Ok(())
}

#[test]
fn test_encode_fails_for_payload_of_wrong_shape() -> anyhow::Result<()> {
    let message = OrderOutboxMessage::new(
        Uuid::new_v4(),
        OutboxMessageType::PaymentRequest,
        &serde_json::json!({ "unexpected": true }),
    )?;

    assert!(encode(&message).is_err());

    Ok(())
}
