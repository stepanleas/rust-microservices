use crate::mappers::{
    payment_created_event_to_avro, payment_request_to_command, payment_response_from_payload,
    payment_updated_event_to_avro,
};
use application::outbox::PaymentResponsePayload;
use domain::aggregates::payment::{Payment, PaymentId};
use domain::enums::PaymentStatus;
use domain::events::{PaymentCreatedEvent, PaymentUpdatedEvent};
use rust_decimal::Decimal;
use shared::contracts::payment::PaymentRequestAvroModel;
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

fn payment() -> Payment {
    Payment::new(
        PaymentId::new(),
        OrderId::new(),
        CustomerId::new(),
        Decimal::new(15000, 2),
        PaymentStatus::Cancelled,
    )
}

fn payment_request(order_id: &str, price: &str) -> PaymentRequestAvroModel {
    PaymentRequestAvroModel {
        id: Uuid::new_v4().to_string(),
        saga_id: Uuid::new_v4().to_string(),
        order_id: order_id.into(),
        customer_id: Uuid::new_v4().to_string(),
        price: price.into(),
        created_at: "2026-10-07T12:00:00Z".into(),
    }
}

#[test]
fn test_payment_response_from_payload() {
    let saga_id = Uuid::new_v4();
    let payload = payload();

    let model = payment_response_from_payload(saga_id, payload.clone());

    assert!(Uuid::parse_str(&model.id).is_ok());
    assert_eq!(saga_id.to_string(), model.saga_id);
    assert_eq!(payload.payment_id.to_string(), model.payment_id);
    assert_eq!(payload.order_id.to_string(), model.order_id);
    assert_eq!(payload.customer_id.to_string(), model.customer_id);
    assert_eq!("150.00", model.price);
    assert_eq!("FAILED", model.payment_status);
    assert_eq!(payload.failure_messages, model.failure_messages);
    assert_eq!(payload.created_at, model.created_at);
}

#[test]
fn test_payment_request_to_command() -> anyhow::Result<()> {
    let order_id = Uuid::new_v4();
    let request = payment_request(&order_id.to_string(), "150.00");

    let command = payment_request_to_command(&request)?;

    assert_eq!(request.saga_id, command.saga_id().to_string());
    assert_eq!(order_id, command.order_id());
    assert_eq!(request.customer_id, command.customer_id().to_string());
    assert_eq!(Decimal::new(15000, 2), command.price());
    Ok(())
}

#[test]
fn test_payment_request_to_command_with_invalid_uuid() {
    let error = payment_request_to_command(&payment_request("order-123", "1.00")).unwrap_err();

    assert!(
        error
            .to_string()
            .starts_with("Failed to parse order_id from Avro model value 'order-123'"),
        "unexpected message: {error}"
    );
}

#[test]
fn test_payment_created_and_updated_events_to_avro() {
    let payment = payment();

    let created = PaymentCreatedEvent::new(payment.clone());
    let model = payment_created_event_to_avro(&created);
    assert_eq!(created.id().to_string(), model.id);
    assert_eq!(payment.id().to_string(), model.payment.id);
    assert_eq!(payment.order_id().to_string(), model.payment.order_id);
    assert_eq!(payment.customer_id().to_string(), model.payment.customer_id);
    assert_eq!("150.00", model.payment.price);
    assert_eq!("CANCELLED", model.payment.payment_status);

    let updated = PaymentUpdatedEvent::new(payment.clone());
    let model = payment_updated_event_to_avro(&updated);
    assert_eq!(updated.id().to_string(), model.id);
    assert_eq!(payment.id().to_string(), model.payment.id);
    assert_eq!(updated.created_at().to_string(), model.created_at);
}
