use super::avro_message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomerAvroModel {
    pub id: String,
    pub user_name: String,
    pub first_name: String,
    pub last_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomerCreatedEventAvroModel {
    pub id: String,
    pub customer: CustomerAvroModel,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomerUpdatedEventAvroModel {
    pub id: String,
    pub customer: CustomerAvroModel,
    pub created_at: String,
}

avro_message!(
    CustomerCreatedEventAvroModel,
    "customer_created_event_avro_model.avsc"
);
avro_message!(
    CustomerUpdatedEventAvroModel,
    "customer_updated_event_avro_model.avsc"
);
