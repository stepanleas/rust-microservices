//! Domain events -> wire contracts (`shared::contracts::catalog`).

use domain::aggregates::product::Product;
use domain::events::{ProductCreatedEvent, ProductDeletedEvent, ProductUpdatedEvent};
use shared::contracts::catalog::{
    ProductAvroModel, ProductCreatedEventAvroModel, ProductDeletedEventAvroModel,
    ProductUpdatedEventAvroModel,
};

/// Consumers (order, restaurant) parse the price as a plain decimal, e.g. "25.00", not "$25.00".
fn product_to_avro(product: &Product) -> ProductAvroModel {
    ProductAvroModel {
        id: product.id().to_string(),
        title: product.title().to_string(),
        quantity: product.quantity(),
        price: format!("{:.2}", product.price().amount()),
    }
}

pub fn product_created_to_avro(event: &ProductCreatedEvent) -> ProductCreatedEventAvroModel {
    ProductCreatedEventAvroModel {
        id: event.id().to_string(),
        product: product_to_avro(event.product()),
        created_at: event.created_at().to_string(),
    }
}

pub fn product_updated_to_avro(event: &ProductUpdatedEvent) -> ProductUpdatedEventAvroModel {
    ProductUpdatedEventAvroModel {
        id: event.id().to_string(),
        product: product_to_avro(event.product()),
        created_at: event.created_at().to_string(),
    }
}

pub fn product_deleted_to_avro(event: &ProductDeletedEvent) -> ProductDeletedEventAvroModel {
    ProductDeletedEventAvroModel {
        id: event.id().to_string(),
        product_id: event.product_id().to_string(),
        created_at: event.created_at().to_string(),
    }
}

#[cfg(test)]
#[path = "mappers_tests.rs"]
mod tests;
