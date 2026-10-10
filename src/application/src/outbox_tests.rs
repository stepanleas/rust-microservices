use crate::outbox::{OutboxMessageType, PaymentResponsePayload};
use domain::aggregates::payment::Payment;
use rust_decimal::Decimal;
use shared::outbox::MessageKind;
use uuid::Uuid;

#[test]
fn test_outbox_message_type_round_trip() -> anyhow::Result<()> {
    assert_eq!(
        OutboxMessageType::PaymentResponse,
        OutboxMessageType::parse(OutboxMessageType::PaymentResponse.as_str())?
    );
    assert_eq!(
        "PAYMENT_RESPONSE",
        OutboxMessageType::PaymentResponse.as_str()
    );
    assert!(OutboxMessageType::parse("UNKNOWN").is_err());

    Ok(())
}

#[test]
fn test_payment_response_payload_from_payment() {
    let saga_id = Uuid::new_v4();
    let mut payment = Payment::builder().price(Decimal::ZERO).build();
    payment.process();

    let payload = PaymentResponsePayload::from_payment(saga_id, &payment);

    assert_eq!(saga_id, payload.saga_id);
    assert_eq!(*payment.id().as_uuid(), payload.payment_id);
    assert_eq!("0", payload.price);
    assert_eq!("FAILED", payload.payment_status);
    assert_eq!(
        vec!["Total price must be greater than zero!".to_string()],
        payload.failure_messages
    );
}
