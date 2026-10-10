//! Kafka contracts (`shared::contracts`) <-> application DTOs and commands.

use anyhow::Context;
use application::commands::{ApproveOrderCommand, CancelOrderApprovalCommand, OrderProductDto};
use application::dtos::ProductDto;
use rust_decimal::Decimal;
use shared::contracts::catalog::ProductAvroModel;
use shared::contracts::restaurant::{
    RestaurantApprovalCancelRequestAvroModel, RestaurantApprovalProductAvroModel,
    RestaurantApprovalRequestAvroModel,
};
use std::str::FromStr;
use uuid::Uuid;

pub struct ProductMessagingMapper;

impl ProductMessagingMapper {
    pub fn product_avro_model_to_dto(avro_model: &ProductAvroModel) -> anyhow::Result<ProductDto> {
        let id = Self::product_id(&avro_model.id)?;

        let price = Decimal::from_str(&avro_model.price).map_err(|error| {
            anyhow::anyhow!(
                "Failed to convert price '{}' to Decimal for Avro model id '{}': {}",
                avro_model.price,
                avro_model.id,
                error,
            )
        })?;

        Ok(ProductDto {
            id,
            title: avro_model.title.clone(),
            quantity: avro_model.quantity,
            price,
        })
    }

    pub fn product_id(value: &str) -> anyhow::Result<Uuid> {
        Uuid::parse_str(value).map_err(|error| {
            anyhow::anyhow!(
                "Failed to parse ProductId from Avro model id '{}': {}",
                value,
                error,
            )
        })
    }
}

pub struct RestaurantApprovalMessagingMapper;

impl RestaurantApprovalMessagingMapper {
    pub fn approval_request_to_command(
        avro_model: &RestaurantApprovalRequestAvroModel,
    ) -> anyhow::Result<ApproveOrderCommand> {
        Ok(ApproveOrderCommand::new(
            parse_uuid("saga id", &avro_model.saga_id)?,
            parse_uuid("order id", &avro_model.order_id)?,
            parse_uuid("customer id", &avro_model.customer_id)?,
            Decimal::from_str(&avro_model.price)
                .with_context(|| format!("Invalid price '{}'", avro_model.price))?,
            products(&avro_model.products)?,
        ))
    }

    pub fn approval_cancel_request_to_command(
        avro_model: &RestaurantApprovalCancelRequestAvroModel,
    ) -> anyhow::Result<CancelOrderApprovalCommand> {
        Ok(CancelOrderApprovalCommand::new(
            parse_uuid("saga id", &avro_model.saga_id)?,
            parse_uuid("order id", &avro_model.order_id)?,
            products(&avro_model.products)?,
        ))
    }
}

fn products(
    products: &[RestaurantApprovalProductAvroModel],
) -> anyhow::Result<Vec<OrderProductDto>> {
    products
        .iter()
        .map(|product| {
            Ok(OrderProductDto::new(
                parse_uuid("product id", &product.id)?,
                product.quantity,
            ))
        })
        .collect()
}

fn parse_uuid(field: &str, value: &str) -> anyhow::Result<Uuid> {
    Uuid::parse_str(value).with_context(|| format!("Invalid {field} '{value}'"))
}
