//! Conversions between the Kafka contracts (`shared::contracts`) and the application types.

use application::outbox::{
    OrderApprovalCancelEventPayload, OrderApprovalEventPayload, OrderApprovalEventProduct,
    OrderPaymentEventPayload,
};
use application::saga::dtos::{
    OrderApprovalStatus, PaymentResponse, PaymentResponseStatus, RestaurantApprovalResponse,
};
use domain::aggregates::customer::Customer;
use domain::aggregates::product::Product;
use rust_decimal::Decimal;
use rusty_money::{Money, iso};
use shared::contracts::catalog::ProductAvroModel;
use shared::contracts::customer::CustomerAvroModel;
use shared::contracts::payment::{PaymentRequestAvroModel, PaymentResponseAvroModel};
use shared::contracts::restaurant::{
    RestaurantApprovalCancelRequestAvroModel, RestaurantApprovalProductAvroModel,
    RestaurantApprovalRequestAvroModel, RestaurantApprovalResponseAvroModel,
};
use shared::domain::value_objects::{CustomerId, ProductId};
use std::str::FromStr;
use uuid::Uuid;

fn parse_uuid(field: &str, value: &str) -> anyhow::Result<Uuid> {
    Uuid::parse_str(value)
        .map_err(|error| anyhow::anyhow!("Invalid {} '{}': {}", field, value, error))
}

pub fn parse_customer_id(value: &str) -> anyhow::Result<CustomerId> {
    CustomerId::from_str(value).map_err(|error| {
        anyhow::anyhow!("Failed to parse CustomerId from Avro model id '{value}': {error}")
    })
}

pub fn parse_product_id(value: &str) -> anyhow::Result<ProductId> {
    ProductId::from_str(value).map_err(|error| {
        anyhow::anyhow!("Failed to parse ProductId from Avro model id '{value}': {error}")
    })
}

pub fn customer_from_avro(model: &CustomerAvroModel) -> anyhow::Result<Customer> {
    Ok(Customer::builder()
        .id(parse_customer_id(&model.id)?)
        .user_name(model.user_name.clone())
        .first_name(model.first_name.clone())
        .last_name(model.last_name.clone())
        .build())
}

pub fn product_from_avro(model: &ProductAvroModel) -> anyhow::Result<Product> {
    let product_id = parse_product_id(&model.id)?;
    let price = Decimal::from_str(&model.price).map_err(|error| {
        anyhow::anyhow!(
            "Failed to convert price '{}' to Decimal for Avro model id '{}': {}",
            model.price,
            model.id,
            error,
        )
    })?;

    Ok(Product::builder()
        .id(product_id)
        .title(model.title.clone())
        .quantity(model.quantity)
        .price(Money::from_decimal(price, iso::USD))
        .build())
}

pub fn restaurant_approval_response_from_avro(
    model: RestaurantApprovalResponseAvroModel,
) -> anyhow::Result<RestaurantApprovalResponse> {
    Ok(RestaurantApprovalResponse {
        saga_id: parse_uuid("saga_id", &model.saga_id)?,
        order_id: parse_uuid("order_id", &model.order_id)?,
        status: OrderApprovalStatus::try_from(model.order_approval_status.as_str())?,
        failure_messages: model.failure_messages,
    })
}

pub fn payment_response_from_avro(
    model: PaymentResponseAvroModel,
) -> anyhow::Result<PaymentResponse> {
    Ok(PaymentResponse {
        saga_id: parse_uuid("saga_id", &model.saga_id)?,
        order_id: parse_uuid("order_id", &model.order_id)?,
        payment_id: parse_uuid("payment_id", &model.payment_id)?,
        status: PaymentResponseStatus::try_from(model.payment_status.as_str())?,
        failure_messages: model.failure_messages,
    })
}

fn products_to_avro(
    products: Vec<OrderApprovalEventProduct>,
) -> Vec<RestaurantApprovalProductAvroModel> {
    products
        .into_iter()
        .map(|product| RestaurantApprovalProductAvroModel {
            id: product.id.to_string(),
            quantity: product.quantity,
        })
        .collect()
}

pub fn restaurant_approval_request_to_avro(
    payload: OrderApprovalEventPayload,
) -> RestaurantApprovalRequestAvroModel {
    RestaurantApprovalRequestAvroModel {
        id: payload.id.to_string(),
        saga_id: payload.saga_id.to_string(),
        order_id: payload.order_id.to_string(),
        customer_id: payload.customer_id.to_string(),
        price: payload.price,
        products: products_to_avro(payload.products),
        created_at: payload.created_at,
    }
}

pub fn restaurant_approval_cancel_request_to_avro(
    payload: OrderApprovalCancelEventPayload,
) -> RestaurantApprovalCancelRequestAvroModel {
    RestaurantApprovalCancelRequestAvroModel {
        id: payload.id.to_string(),
        saga_id: payload.saga_id.to_string(),
        order_id: payload.order_id.to_string(),
        products: products_to_avro(payload.products),
        created_at: payload.created_at,
    }
}

pub fn payment_request_to_avro(payload: OrderPaymentEventPayload) -> PaymentRequestAvroModel {
    PaymentRequestAvroModel {
        id: payload.id.to_string(),
        saga_id: payload.saga_id.to_string(),
        order_id: payload.order_id.to_string(),
        customer_id: payload.customer_id.to_string(),
        price: payload.price,
        created_at: payload.created_at,
    }
}
