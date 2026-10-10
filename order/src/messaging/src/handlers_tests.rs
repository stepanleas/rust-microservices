use crate::handlers::OrderMessageHandler;
use application::ports::r#in::messages::{
    MockCustomerMessageListener, MockPaymentResponseMessageListener, MockProductMessageListener,
    MockRestaurantApprovalResponseMessageListener,
};
use application::saga::dtos::{OrderApprovalStatus, PaymentResponseStatus};
use shared::contracts::AvroMessage;
use shared::contracts::catalog::{
    ProductAvroModel, ProductCreatedEventAvroModel, ProductDeletedEventAvroModel,
    ProductUpdatedEventAvroModel,
};
use shared::contracts::customer::{
    CustomerAvroModel, CustomerCreatedEventAvroModel, CustomerUpdatedEventAvroModel,
};
use shared::contracts::payment::PaymentResponseAvroModel;
use shared::contracts::restaurant::RestaurantApprovalResponseAvroModel;
use shared::contracts::topics;
use shared::error::AppError;
use shared::kafka::MessageHandler;
use std::sync::Arc;

const EVENT_ID: &str = "eb0163b3-cf88-4053-8c96-986b4f5b9f2e";
const ENTITY_ID: &str = "0411a4a9-1edb-4180-9556-800cb2b84721";
const SAGA_ID: &str = "0411a4a9-1edb-4180-9556-800cb2b84721";
const ORDER_ID: &str = "eb0163b3-cf88-4053-8c96-986b4f5b9f2e";
const PAYMENT_ID: &str = "5d0b5f8e-7f1e-4a1d-8a43-3a4a0b0e6f10";
const CREATED_AT: &str = "2025-01-01T12:00:00Z";

#[derive(Default)]
struct Listeners {
    customer: MockCustomerMessageListener,
    product: MockProductMessageListener,
    restaurant_approval: MockRestaurantApprovalResponseMessageListener,
    payment: MockPaymentResponseMessageListener,
}

impl Listeners {
    fn handler(self) -> OrderMessageHandler {
        OrderMessageHandler::new(
            Arc::new(self.customer),
            Arc::new(self.product),
            Arc::new(self.restaurant_approval),
            Arc::new(self.payment),
        )
    }
}

fn customer(id: &str) -> CustomerAvroModel {
    CustomerAvroModel {
        id: id.into(),
        user_name: "Artellas".into(),
        first_name: "Mike".into(),
        last_name: "Dane".into(),
    }
}

fn product(id: &str) -> ProductAvroModel {
    ProductAvroModel {
        id: id.into(),
        title: "Laptop".into(),
        quantity: 10,
        price: "99.99".into(),
    }
}

fn restaurant_approval_response(
    status: &str,
    failure_messages: Vec<String>,
) -> RestaurantApprovalResponseAvroModel {
    RestaurantApprovalResponseAvroModel {
        id: "3f1b6c1e-6c2e-4bb3-9b6b-1b0b0c7c2a11".into(),
        saga_id: SAGA_ID.into(),
        order_id: ORDER_ID.into(),
        order_approval_status: status.into(),
        failure_messages,
        created_at: CREATED_AT.into(),
    }
}

fn payment_response(status: &str) -> PaymentResponseAvroModel {
    PaymentResponseAvroModel {
        id: "3f1b6c1e-6c2e-4bb3-9b6b-1b0b0c7c2a11".into(),
        saga_id: SAGA_ID.into(),
        payment_id: PAYMENT_ID.into(),
        order_id: ORDER_ID.into(),
        customer_id: "9b2a3c4d-1e2f-4a5b-8c7d-6e5f4a3b2c1d".into(),
        price: "60.00".into(),
        payment_status: status.into(),
        failure_messages: match status {
            "COMPLETED" => vec![],
            _ => vec!["Payment declined".into()],
        },
        created_at: CREATED_AT.into(),
    }
}

#[test]
fn test_subscribes_to_all_consumed_topics() {
    let topics = Listeners::default().handler().topics();

    assert_eq!(
        vec![
            topics::CUSTOMER_CREATED,
            topics::CUSTOMER_UPDATED,
            topics::PRODUCT_CREATED,
            topics::PRODUCT_UPDATED,
            topics::PRODUCT_DELETED,
            topics::RESTAURANT_APPROVAL_RESPONSE,
            topics::PAYMENT_RESPONSE,
        ],
        topics
    );
}

#[test]
fn test_unknown_topic_is_an_error() {
    let result = Listeners::default().handler().handle("unknown-topic", b"");

    assert_eq!(
        "No handler for topic unknown-topic",
        result.unwrap_err().to_string()
    );
}

#[test]
fn test_garbage_payload_is_an_error() {
    let handler = Listeners::default().handler();

    for topic in handler.topics() {
        assert!(handler.handle(topic, b"not avro").is_err(), "{topic}");
    }
}

#[test]
fn test_customer_created() -> anyhow::Result<()> {
    let mut listeners = Listeners::default();
    listeners
        .customer
        .expect_customer_created()
        .once()
        .withf(|customer| {
            customer.id().to_string() == ENTITY_ID
                && customer.user_name() == "Artellas"
                && customer.first_name() == "Mike"
                && customer.last_name() == "Dane"
        })
        .returning(|_| Ok(()));

    let payload = CustomerCreatedEventAvroModel {
        id: EVENT_ID.into(),
        customer: customer(ENTITY_ID),
        created_at: CREATED_AT.into(),
    }
    .to_avro_bytes()?;

    listeners
        .handler()
        .handle(topics::CUSTOMER_CREATED, &payload)
}

#[test]
fn test_customer_updated() -> anyhow::Result<()> {
    let mut listeners = Listeners::default();
    listeners
        .customer
        .expect_customer_updated()
        .once()
        .withf(|customer| customer.id().to_string() == ENTITY_ID)
        .returning(|_| Ok(()));

    let payload = CustomerUpdatedEventAvroModel {
        id: EVENT_ID.into(),
        customer: customer(ENTITY_ID),
        created_at: CREATED_AT.into(),
    }
    .to_avro_bytes()?;

    listeners
        .handler()
        .handle(topics::CUSTOMER_UPDATED, &payload)
}

#[test]
fn test_customer_with_invalid_id_is_an_error() -> anyhow::Result<()> {
    let mut listeners = Listeners::default();
    listeners.customer.expect_customer_created().never();

    let payload = CustomerCreatedEventAvroModel {
        id: EVENT_ID.into(),
        customer: customer("customer-123"),
        created_at: CREATED_AT.into(),
    }
    .to_avro_bytes()?;

    let message = listeners
        .handler()
        .handle(topics::CUSTOMER_CREATED, &payload)
        .unwrap_err()
        .to_string();

    assert!(
        message.starts_with(
            "Failed to parse CustomerId from Avro model id 'customer-123': invalid character"
        ),
        "{message}",
    );

    Ok(())
}

#[test]
fn test_listener_error_is_propagated() -> anyhow::Result<()> {
    let mut listeners = Listeners::default();
    listeners
        .customer
        .expect_customer_created()
        .once()
        .returning(|_| Err(AppError::Internal(anyhow::anyhow!("database is down"))));

    let payload = CustomerCreatedEventAvroModel {
        id: EVENT_ID.into(),
        customer: customer(ENTITY_ID),
        created_at: CREATED_AT.into(),
    }
    .to_avro_bytes()?;

    let result = listeners
        .handler()
        .handle(topics::CUSTOMER_CREATED, &payload);

    assert_eq!("database is down", result.unwrap_err().to_string());

    Ok(())
}

#[test]
fn test_product_created() -> anyhow::Result<()> {
    let mut listeners = Listeners::default();
    listeners
        .product
        .expect_product_created()
        .once()
        .withf(|product| {
            product.id().to_string() == ENTITY_ID
                && product.title() == "Laptop"
                && product.quantity() == 10
                && product.price().to_string() == "$99.99"
        })
        .returning(|_| Ok(()));

    let payload = ProductCreatedEventAvroModel {
        id: EVENT_ID.into(),
        product: product(ENTITY_ID),
        created_at: CREATED_AT.into(),
    }
    .to_avro_bytes()?;

    listeners
        .handler()
        .handle(topics::PRODUCT_CREATED, &payload)
}

#[test]
fn test_product_updated() -> anyhow::Result<()> {
    let mut listeners = Listeners::default();
    listeners
        .product
        .expect_product_updated()
        .once()
        .withf(|product| {
            product.id().to_string() == ENTITY_ID && product.price().to_string() == "$99.99"
        })
        .returning(|_| Ok(()));

    let payload = ProductUpdatedEventAvroModel {
        id: EVENT_ID.into(),
        product: product(ENTITY_ID),
        created_at: CREATED_AT.into(),
    }
    .to_avro_bytes()?;

    listeners
        .handler()
        .handle(topics::PRODUCT_UPDATED, &payload)
}

#[test]
fn test_product_with_invalid_id_is_an_error() -> anyhow::Result<()> {
    let mut listeners = Listeners::default();
    listeners.product.expect_product_updated().never();

    let payload = ProductUpdatedEventAvroModel {
        id: EVENT_ID.into(),
        product: product("product-123"),
        created_at: CREATED_AT.into(),
    }
    .to_avro_bytes()?;

    let message = listeners
        .handler()
        .handle(topics::PRODUCT_UPDATED, &payload)
        .unwrap_err()
        .to_string();

    assert!(
        message.starts_with(
            "Failed to parse ProductId from Avro model id 'product-123': invalid character"
        ),
        "{message}",
    );

    Ok(())
}

#[test]
fn test_product_deleted() -> anyhow::Result<()> {
    let mut listeners = Listeners::default();
    listeners
        .product
        .expect_product_deleted()
        .once()
        .withf(|product_id| product_id.to_string() == ENTITY_ID)
        .returning(|_| Ok(()));

    let payload = ProductDeletedEventAvroModel {
        id: EVENT_ID.into(),
        product_id: ENTITY_ID.into(),
        created_at: CREATED_AT.into(),
    }
    .to_avro_bytes()?;

    listeners
        .handler()
        .handle(topics::PRODUCT_DELETED, &payload)
}

#[test]
fn test_product_deleted_with_invalid_id_is_an_error() -> anyhow::Result<()> {
    let mut listeners = Listeners::default();
    listeners.product.expect_product_deleted().never();

    let payload = ProductDeletedEventAvroModel {
        id: EVENT_ID.into(),
        product_id: "product-123".into(),
        created_at: CREATED_AT.into(),
    }
    .to_avro_bytes()?;

    assert!(
        listeners
            .handler()
            .handle(topics::PRODUCT_DELETED, &payload)
            .is_err()
    );

    Ok(())
}

#[test]
fn test_approved_response_is_forwarded() -> anyhow::Result<()> {
    let mut listeners = Listeners::default();
    listeners
        .restaurant_approval
        .expect_order_approved()
        .once()
        .withf(|response| {
            response.saga_id.to_string() == SAGA_ID
                && response.order_id.to_string() == ORDER_ID
                && response.status == OrderApprovalStatus::Approved
                && response.failure_messages.is_empty()
        })
        .returning(|_| Ok(()));
    listeners
        .restaurant_approval
        .expect_order_rejected()
        .never();

    let payload = restaurant_approval_response("APPROVED", vec![]).to_avro_bytes()?;

    listeners
        .handler()
        .handle(topics::RESTAURANT_APPROVAL_RESPONSE, &payload)
}

#[test]
fn test_rejected_response_is_forwarded_with_failure_messages() -> anyhow::Result<()> {
    let mut listeners = Listeners::default();
    listeners
        .restaurant_approval
        .expect_order_approved()
        .never();
    listeners
        .restaurant_approval
        .expect_order_rejected()
        .once()
        .withf(|response| {
            response.status == OrderApprovalStatus::Rejected
                && response.failure_messages == vec!["Insufficient stock".to_string()]
        })
        .returning(|_| Ok(()));

    let payload = restaurant_approval_response("REJECTED", vec!["Insufficient stock".into()])
        .to_avro_bytes()?;

    listeners
        .handler()
        .handle(topics::RESTAURANT_APPROVAL_RESPONSE, &payload)
}

#[test]
fn test_unknown_approval_status_is_an_error() -> anyhow::Result<()> {
    let payload = restaurant_approval_response("MAYBE", vec![]).to_avro_bytes()?;

    let result = Listeners::default()
        .handler()
        .handle(topics::RESTAURANT_APPROVAL_RESPONSE, &payload);

    assert_eq!(
        "Unknown order approval status: MAYBE",
        result.unwrap_err().to_string(),
    );

    Ok(())
}

#[test]
fn test_invalid_saga_id_is_an_error() -> anyhow::Result<()> {
    let mut invalid = restaurant_approval_response("APPROVED", vec![]);
    invalid.saga_id = "saga-123".into();

    let result = Listeners::default().handler().handle(
        topics::RESTAURANT_APPROVAL_RESPONSE,
        &invalid.to_avro_bytes()?,
    );

    assert!(
        result
            .unwrap_err()
            .to_string()
            .starts_with("Invalid saga_id 'saga-123'")
    );

    Ok(())
}

#[test]
fn test_completed_payment_is_forwarded() -> anyhow::Result<()> {
    let mut listeners = Listeners::default();
    listeners
        .payment
        .expect_payment_completed()
        .once()
        .withf(|response| {
            response.payment_id.to_string() == PAYMENT_ID
                && response.status == PaymentResponseStatus::Completed
        })
        .returning(|_| Ok(()));
    listeners.payment.expect_payment_failed().never();

    let payload = payment_response("COMPLETED").to_avro_bytes()?;

    listeners
        .handler()
        .handle(topics::PAYMENT_RESPONSE, &payload)
}

#[test]
fn test_failed_and_cancelled_payments_are_forwarded_as_failures() -> anyhow::Result<()> {
    let mut listeners = Listeners::default();
    listeners.payment.expect_payment_completed().never();
    listeners
        .payment
        .expect_payment_failed()
        .times(2)
        .withf(|response| response.failure_messages == vec!["Payment declined".to_string()])
        .returning(|_| Ok(()));

    let handler = listeners.handler();
    handler.handle(
        topics::PAYMENT_RESPONSE,
        &payment_response("FAILED").to_avro_bytes()?,
    )?;
    handler.handle(
        topics::PAYMENT_RESPONSE,
        &payment_response("CANCELLED").to_avro_bytes()?,
    )?;

    Ok(())
}
