use crate::handlers::PaymentMessageHandler;
use application::ports::r#in::messages::MockPaymentRequestMessageListener;
use rust_decimal::Decimal;
use shared::contracts::AvroMessage;
use shared::contracts::payment::PaymentRequestAvroModel;
use shared::contracts::topics;
use shared::error::AppError;
use shared::kafka::MessageHandler;
use std::sync::Arc;
use uuid::Uuid;

const SAGA_ID: &str = "9a4b2c1e-2a51-4c55-8d8b-0f8a3f0d6a11";
const ORDER_ID: &str = "0411a4a9-1edb-4180-9556-800cb2b84721";
const CUSTOMER_ID: &str = "eb0163b3-cf88-4053-8c96-986b4f5b9f2e";

fn payment_request(order_id: &str, price: &str) -> anyhow::Result<Vec<u8>> {
    PaymentRequestAvroModel {
        id: Uuid::new_v4().to_string(),
        saga_id: SAGA_ID.into(),
        order_id: order_id.into(),
        customer_id: CUSTOMER_ID.into(),
        price: price.into(),
        created_at: "2026-10-07T12:00:00Z".into(),
    }
    .to_avro_bytes()
}

fn handler(listener: MockPaymentRequestMessageListener) -> PaymentMessageHandler {
    PaymentMessageHandler::new(Arc::new(listener))
}

#[test]
fn test_handle_payment_request() -> anyhow::Result<()> {
    let mut mock_listener = MockPaymentRequestMessageListener::new();
    mock_listener
        .expect_payment_requested()
        .once()
        .withf(|command| {
            command.saga_id().to_string() == SAGA_ID
                && command.order_id().to_string() == ORDER_ID
                && command.customer_id().to_string() == CUSTOMER_ID
                && command.price() == Decimal::new(15000, 2)
        })
        .returning(|_| Ok(()));

    handler(mock_listener).handle(
        topics::PAYMENT_REQUEST,
        &payment_request(ORDER_ID, "150.00")?,
    )?;

    Ok(())
}

#[test]
fn test_handle_payment_request_with_invalid_order_id() -> anyhow::Result<()> {
    let mut mock_listener = MockPaymentRequestMessageListener::new();
    mock_listener.expect_payment_requested().never();

    let result = handler(mock_listener).handle(
        topics::PAYMENT_REQUEST,
        &payment_request("order-123", "150.00")?,
    );

    let message = result.unwrap_err().to_string();
    assert!(
        message.starts_with("Failed to parse order_id from Avro model value 'order-123'"),
        "unexpected message: {message}"
    );

    Ok(())
}

#[test]
fn test_handle_payment_request_with_invalid_price() -> anyhow::Result<()> {
    let mut mock_listener = MockPaymentRequestMessageListener::new();
    mock_listener.expect_payment_requested().never();

    let result =
        handler(mock_listener).handle(topics::PAYMENT_REQUEST, &payment_request(ORDER_ID, "abc")?);

    let message = result.unwrap_err().to_string();
    assert!(
        message.starts_with(&format!(
            "Failed to parse price 'abc' of payment request for order '{ORDER_ID}'"
        )),
        "unexpected message: {message}"
    );

    Ok(())
}

#[test]
fn test_handle_payment_request_propagates_listener_error() -> anyhow::Result<()> {
    let mut mock_listener = MockPaymentRequestMessageListener::new();
    mock_listener
        .expect_payment_requested()
        .once()
        .returning(|_| Err(AppError::Internal(anyhow::anyhow!("database is down"))));

    let result =
        handler(mock_listener).handle(topics::PAYMENT_REQUEST, &payment_request(ORDER_ID, "1.00")?);

    assert_eq!("database is down", result.unwrap_err().to_string());

    Ok(())
}

#[test]
fn test_handle_payment_request_with_invalid_payload() {
    let handler = handler(MockPaymentRequestMessageListener::new());

    assert!(
        handler
            .handle(topics::PAYMENT_REQUEST, b"not avro")
            .is_err()
    );
}

#[test]
fn test_handler_consumes_only_payment_requests() {
    let handler = handler(MockPaymentRequestMessageListener::new());

    assert_eq!(vec![topics::PAYMENT_REQUEST], handler.topics());
    assert!(
        handler
            .handle(topics::PAYMENT_RESPONSE, b"")
            .unwrap_err()
            .to_string()
            .contains("is not consumed")
    );
}
