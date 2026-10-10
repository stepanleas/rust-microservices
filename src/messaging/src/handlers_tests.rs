use crate::handlers::RestaurantMessageHandler;
use apache_avro::types::Value;
use application::ports::r#in::messages::{
    MockProductMessageListener, MockRestaurantApprovalMessageListener,
};
use rust_decimal::dec;
use shared::contracts::AvroMessage;
use shared::contracts::catalog::{
    ProductAvroModel, ProductDeletedEventAvroModel, ProductUpdatedEventAvroModel,
};
use shared::contracts::restaurant::{
    RestaurantApprovalCancelRequestAvroModel, RestaurantApprovalProductAvroModel,
    RestaurantApprovalRequestAvroModel,
};
use shared::contracts::topics;
use shared::error::AppError;
use shared::kafka::MessageHandler;
use std::sync::Arc;

const SAGA_ID: &str = "5b0c50f1-7ab4-4d0b-9a5d-5f6a2f0e8b11";
const ORDER_ID: &str = "8f0b3a52-5d1e-4c39-8f4d-3c1d2e1f0a22";
const CUSTOMER_ID: &str = "0411a4a9-1edb-4180-9556-800cb2b84721";
const PRODUCT_ID: &str = "c2a4f7e8-1b2c-4d5e-8f90-a1b2c3d4e5f6";

fn handler(
    product_listener: MockProductMessageListener,
    approval_listener: MockRestaurantApprovalMessageListener,
) -> RestaurantMessageHandler {
    RestaurantMessageHandler::new(Arc::new(product_listener), Arc::new(approval_listener))
}

/// Encodes a record with the raw Avro schema (as another service would), not through the model.
fn encode(schema: &str, value: Value) -> anyhow::Result<Vec<u8>> {
    let schema = apache_avro::Schema::parse_str(schema)?;
    let mut writer = apache_avro::Writer::new(&schema, Vec::new())?;
    writer.append_value(value)?;
    writer.flush()?;
    Ok(writer.into_inner()?)
}

fn product_created(product_id: &str) -> anyhow::Result<Vec<u8>> {
    encode(
        <shared::contracts::catalog::ProductCreatedEventAvroModel as AvroMessage>::SCHEMA,
        Value::Record(vec![
            ("id".into(), Value::String(uuid::Uuid::new_v4().to_string())),
            (
                "product".into(),
                Value::Record(vec![
                    ("id".into(), Value::String(product_id.into())),
                    ("title".into(), Value::String("Pizza".into())),
                    ("quantity".into(), Value::Int(12)),
                    ("price".into(), Value::String("12.5".into())),
                ]),
            ),
            (
                "created_at".into(),
                Value::String("2025-01-01T12:00:00Z".into()),
            ),
        ]),
    )
}

fn approval_request(order_id: &str) -> anyhow::Result<Vec<u8>> {
    RestaurantApprovalRequestAvroModel {
        id: uuid::Uuid::new_v4().to_string(),
        saga_id: SAGA_ID.into(),
        order_id: order_id.into(),
        customer_id: CUSTOMER_ID.into(),
        price: "150.00".into(),
        products: vec![RestaurantApprovalProductAvroModel {
            id: PRODUCT_ID.into(),
            quantity: 3,
        }],
        created_at: "2025-01-01T12:00:00Z".into(),
    }
    .to_avro_bytes()
}

fn cancel_request(order_id: &str) -> anyhow::Result<Vec<u8>> {
    encode(
        RestaurantApprovalCancelRequestAvroModel::SCHEMA,
        Value::Record(vec![
            ("id".into(), Value::String(uuid::Uuid::new_v4().to_string())),
            ("saga_id".into(), Value::String(SAGA_ID.into())),
            ("order_id".into(), Value::String(order_id.into())),
            (
                "products".into(),
                Value::Array(vec![Value::Record(vec![
                    ("id".into(), Value::String(PRODUCT_ID.into())),
                    ("quantity".into(), Value::Int(4)),
                ])]),
            ),
            (
                "created_at".into(),
                Value::String("2025-01-01T12:00:00Z".into()),
            ),
        ]),
    )
}

#[test]
fn test_topics_cover_every_consumed_message() {
    let handler = handler(
        MockProductMessageListener::new(),
        MockRestaurantApprovalMessageListener::new(),
    );

    assert_eq!(
        vec![
            topics::PRODUCT_CREATED,
            topics::PRODUCT_UPDATED,
            topics::PRODUCT_DELETED,
            topics::RESTAURANT_APPROVAL_REQUEST,
            topics::RESTAURANT_APPROVAL_CANCEL_REQUEST,
        ],
        handler.topics(),
    );
}

#[test]
fn test_unknown_topic_is_an_error() {
    let handler = handler(
        MockProductMessageListener::new(),
        MockRestaurantApprovalMessageListener::new(),
    );

    assert_eq!(
        "Unexpected topic: payment-request",
        handler
            .handle(topics::PAYMENT_REQUEST, b"")
            .unwrap_err()
            .to_string(),
    );
}

#[test]
fn test_product_created() -> anyhow::Result<()> {
    let mut product_listener = MockProductMessageListener::new();
    product_listener
        .expect_product_created()
        .once()
        .withf(|product| {
            product.id.to_string() == PRODUCT_ID
                && product.title == "Pizza"
                && product.quantity == 12
                && product.price == dec!(12.5)
        })
        .returning(|_| Ok(()));

    handler(
        product_listener,
        MockRestaurantApprovalMessageListener::new(),
    )
    .handle(topics::PRODUCT_CREATED, &product_created(PRODUCT_ID)?)
}

#[test]
fn test_product_created_with_invalid_id() -> anyhow::Result<()> {
    let mut product_listener = MockProductMessageListener::new();
    product_listener.expect_product_created().never();

    let result = handler(
        product_listener,
        MockRestaurantApprovalMessageListener::new(),
    )
    .handle(topics::PRODUCT_CREATED, &product_created("product-123")?);

    assert!(
        result
            .unwrap_err()
            .to_string()
            .starts_with("Failed to parse ProductId from Avro model id 'product-123'")
    );
    Ok(())
}

#[test]
fn test_product_created_propagates_listener_error() -> anyhow::Result<()> {
    let mut product_listener = MockProductMessageListener::new();
    product_listener
        .expect_product_created()
        .once()
        .returning(|_| Err(AppError::Internal(anyhow::anyhow!("database unavailable"))));

    let result = handler(
        product_listener,
        MockRestaurantApprovalMessageListener::new(),
    )
    .handle(topics::PRODUCT_CREATED, &product_created(PRODUCT_ID)?);

    assert_eq!("database unavailable", result.unwrap_err().to_string());
    Ok(())
}

#[test]
fn test_product_updated() -> anyhow::Result<()> {
    let mut product_listener = MockProductMessageListener::new();
    product_listener
        .expect_product_updated()
        .once()
        .withf(|product| product.id.to_string() == PRODUCT_ID && product.quantity == 7)
        .returning(|_| Ok(()));

    let payload = ProductUpdatedEventAvroModel {
        id: uuid::Uuid::new_v4().to_string(),
        product: ProductAvroModel {
            id: PRODUCT_ID.into(),
            title: "Pizza".into(),
            quantity: 7,
            price: "10.00".into(),
        },
        created_at: "2025-01-01T12:00:00Z".into(),
    }
    .to_avro_bytes()?;

    handler(
        product_listener,
        MockRestaurantApprovalMessageListener::new(),
    )
    .handle(topics::PRODUCT_UPDATED, &payload)
}

#[test]
fn test_product_deleted() -> anyhow::Result<()> {
    let mut product_listener = MockProductMessageListener::new();
    product_listener
        .expect_product_deleted()
        .once()
        .withf(|product_id| product_id.to_string() == PRODUCT_ID)
        .returning(|_| Ok(()));

    let payload = ProductDeletedEventAvroModel {
        id: uuid::Uuid::new_v4().to_string(),
        product_id: PRODUCT_ID.into(),
        created_at: "2025-01-01T12:00:00Z".into(),
    }
    .to_avro_bytes()?;

    handler(
        product_listener,
        MockRestaurantApprovalMessageListener::new(),
    )
    .handle(topics::PRODUCT_DELETED, &payload)
}

#[test]
fn test_restaurant_approval_request() -> anyhow::Result<()> {
    let mut approval_listener = MockRestaurantApprovalMessageListener::new();
    approval_listener
        .expect_approval_requested()
        .once()
        .withf(|command| {
            command.saga_id().to_string() == SAGA_ID
                && command.order_id().to_string() == ORDER_ID
                && command.customer_id().to_string() == CUSTOMER_ID
                && command.price() == dec!(150.00)
                && command.products().len() == 1
                && command.products()[0].product_id().to_string() == PRODUCT_ID
                && command.products()[0].quantity() == 3
        })
        .returning(|_| Ok(()));

    handler(MockProductMessageListener::new(), approval_listener).handle(
        topics::RESTAURANT_APPROVAL_REQUEST,
        &approval_request(ORDER_ID)?,
    )
}

#[test]
fn test_restaurant_approval_request_with_invalid_order_id() -> anyhow::Result<()> {
    let mut approval_listener = MockRestaurantApprovalMessageListener::new();
    approval_listener.expect_approval_requested().never();

    let result = handler(MockProductMessageListener::new(), approval_listener).handle(
        topics::RESTAURANT_APPROVAL_REQUEST,
        &approval_request("order-123")?,
    );

    assert_eq!(
        "Invalid order id 'order-123'",
        result.unwrap_err().to_string()
    );
    Ok(())
}

#[test]
fn test_restaurant_approval_request_with_garbage_payload() {
    let handler = handler(
        MockProductMessageListener::new(),
        MockRestaurantApprovalMessageListener::new(),
    );

    assert!(
        handler
            .handle(topics::RESTAURANT_APPROVAL_REQUEST, b"not avro")
            .is_err()
    );
}

#[test]
fn test_restaurant_approval_cancel_request() -> anyhow::Result<()> {
    let mut approval_listener = MockRestaurantApprovalMessageListener::new();
    approval_listener.expect_approval_requested().never();
    approval_listener
        .expect_approval_cancel_requested()
        .once()
        .withf(|command| {
            command.saga_id().to_string() == SAGA_ID
                && command.order_id().to_string() == ORDER_ID
                && command.products().len() == 1
                && command.products()[0].product_id().to_string() == PRODUCT_ID
                && command.products()[0].quantity() == 4
        })
        .returning(|_| Ok(()));

    handler(MockProductMessageListener::new(), approval_listener).handle(
        topics::RESTAURANT_APPROVAL_CANCEL_REQUEST,
        &cancel_request(ORDER_ID)?,
    )
}

#[test]
fn test_restaurant_approval_cancel_request_propagates_listener_error() -> anyhow::Result<()> {
    let mut approval_listener = MockRestaurantApprovalMessageListener::new();
    approval_listener
        .expect_approval_cancel_requested()
        .once()
        .returning(|_| Err(AppError::Internal(anyhow::anyhow!("database unavailable"))));

    let result = handler(MockProductMessageListener::new(), approval_listener).handle(
        topics::RESTAURANT_APPROVAL_CANCEL_REQUEST,
        &cancel_request(ORDER_ID)?,
    );

    assert_eq!("database unavailable", result.unwrap_err().to_string());
    Ok(())
}

#[test]
fn test_restaurant_approval_cancel_request_with_invalid_order_id() -> anyhow::Result<()> {
    let mut approval_listener = MockRestaurantApprovalMessageListener::new();
    approval_listener.expect_approval_cancel_requested().never();

    let result = handler(MockProductMessageListener::new(), approval_listener).handle(
        topics::RESTAURANT_APPROVAL_CANCEL_REQUEST,
        &cancel_request("order-123")?,
    );

    assert_eq!(
        "Invalid order id 'order-123'",
        result.unwrap_err().to_string()
    );
    Ok(())
}
