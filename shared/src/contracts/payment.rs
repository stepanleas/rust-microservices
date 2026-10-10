use super::avro_message;
use serde::{Deserialize, Serialize};

/// order -> payment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentRequestAvroModel {
    pub id: String,
    pub saga_id: String,
    pub order_id: String,
    pub customer_id: String,
    pub price: String,
    pub created_at: String,
}

/// payment -> order
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentResponseAvroModel {
    pub id: String,
    pub saga_id: String,
    pub payment_id: String,
    pub order_id: String,
    pub customer_id: String,
    pub price: String,
    pub payment_status: String,
    pub failure_messages: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentAvroModel {
    pub id: String,
    pub order_id: String,
    pub customer_id: String,
    pub price: String,
    pub payment_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentCreatedEventAvroModel {
    pub id: String,
    pub payment: PaymentAvroModel,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentUpdatedEventAvroModel {
    pub id: String,
    pub payment: PaymentAvroModel,
    pub created_at: String,
}

avro_message!(PaymentRequestAvroModel, "payment_request_avro_model.avsc");
avro_message!(PaymentResponseAvroModel, "payment_response_avro_model.avsc");
avro_message!(
    PaymentCreatedEventAvroModel,
    "payment_created_event_avro_model.avsc"
);
avro_message!(
    PaymentUpdatedEventAvroModel,
    "payment_updated_event_avro_model.avsc"
);
