use super::avro_message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductAvroModel {
    pub id: String,
    pub title: String,
    pub quantity: i32,
    pub price: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductCreatedEventAvroModel {
    pub id: String,
    pub product: ProductAvroModel,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductUpdatedEventAvroModel {
    pub id: String,
    pub product: ProductAvroModel,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductDeletedEventAvroModel {
    pub id: String,
    pub product_id: String,
    pub created_at: String,
}

avro_message!(
    ProductCreatedEventAvroModel,
    "product_created_event_avro_model.avsc"
);
avro_message!(
    ProductUpdatedEventAvroModel,
    "product_updated_event_avro_model.avsc"
);
avro_message!(
    ProductDeletedEventAvroModel,
    "product_deleted_event_avro_model.avsc"
);
