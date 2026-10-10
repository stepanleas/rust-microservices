use crate::mappers::{payment_created_event_to_avro, payment_updated_event_to_avro};
use crate::publishers::encode;
use application::outbox::{OutboxMessageType, PaymentOutboxMessage, PaymentResponsePayload};
use domain::aggregates::payment::{Payment, PaymentId};
use domain::enums::PaymentStatus;
use domain::events::{PaymentCreatedEvent, PaymentUpdatedEvent};
use rust_decimal::Decimal;
use shared::contracts::AvroMessage;
use shared::contracts::payment::{
    PaymentCreatedEventAvroModel, PaymentResponseAvroModel, PaymentUpdatedEventAvroModel,
};
use shared::contracts::topics;
use shared::domain::value_objects::{CustomerId, OrderId};
use uuid::Uuid;

fn payload() -> PaymentResponsePayload {
    PaymentResponsePayload {
        saga_id: Uuid::new_v4(),
        payment_id: Uuid::new_v4(),
        order_id: Uuid::new_v4(),
        customer_id: Uuid::new_v4(),
        price: "150.00".into(),
        payment_status: "FAILED".into(),
        failure_messages: vec!["Total price must be greater than zero!".into()],
        created_at: "2026-10-07T12:00:00+00:00".into(),
    }
}

#[test]
fn test_encode_outbox_message_produces_decodable_avro() -> anyhow::Result<()> {
    let payload = payload();
    let saga_id = Uuid::new_v4();
    let message = PaymentOutboxMessage::new(saga_id, OutboxMessageType::PaymentResponse, &payload)?;

    let (topic, key, bytes) = encode(&message)?;

    assert_eq!(topics::PAYMENT_RESPONSE, topic);
    assert_eq!(payload.order_id.to_string(), key);

    let decoded = PaymentResponseAvroModel::from_avro_bytes(&bytes)?;
    assert_eq!(1, decoded.len());
    let decoded = &decoded[0];

    assert!(Uuid::parse_str(&decoded.id).is_ok());
    assert_eq!(saga_id.to_string(), decoded.saga_id);
    assert_eq!(payload.payment_id.to_string(), decoded.payment_id);
    assert_eq!(payload.order_id.to_string(), decoded.order_id);
    assert_eq!(payload.customer_id.to_string(), decoded.customer_id);
    assert_eq!("150.00", decoded.price);
    assert_eq!("FAILED", decoded.payment_status);
    assert_eq!(payload.failure_messages, decoded.failure_messages);
    assert_eq!(payload.created_at, decoded.created_at);

    Ok(())
}

#[test]
fn test_encode_outbox_message_with_invalid_payload() -> anyhow::Result<()> {
    let message = PaymentOutboxMessage::new(
        Uuid::new_v4(),
        OutboxMessageType::PaymentResponse,
        &serde_json::json!({ "unexpected": true }),
    )?;

    assert!(encode(&message).is_err());
    Ok(())
}

#[test]
fn test_payment_events_encode_with_the_shared_schemas() -> anyhow::Result<()> {
    let payment = Payment::new(
        PaymentId::new(),
        OrderId::new(),
        CustomerId::new(),
        Decimal::new(1050, 2),
        PaymentStatus::Completed,
    );

    let created = payment_created_event_to_avro(&PaymentCreatedEvent::new(payment.clone()));
    let decoded = PaymentCreatedEventAvroModel::from_avro_bytes(&created.to_avro_bytes()?)?;
    assert_eq!(vec![created], decoded);

    let updated = payment_updated_event_to_avro(&PaymentUpdatedEvent::new(payment));
    let decoded = PaymentUpdatedEventAvroModel::from_avro_bytes(&updated.to_avro_bytes()?)?;
    assert_eq!(vec![updated], decoded);

    Ok(())
}
