use application::commands::ProcessPaymentCommand;
use application::outbox::PaymentResponsePayload;
use domain::aggregates::payment::Payment;
use domain::events::{PaymentCreatedEvent, PaymentUpdatedEvent};
use rust_decimal::Decimal;
use shared::contracts::payment::{
    PaymentAvroModel, PaymentCreatedEventAvroModel, PaymentRequestAvroModel,
    PaymentResponseAvroModel, PaymentUpdatedEventAvroModel,
};
use std::str::FromStr;
use uuid::Uuid;

pub fn payment_request_to_command(
    avro_model: &PaymentRequestAvroModel,
) -> anyhow::Result<ProcessPaymentCommand> {
    let price = Decimal::from_str(&avro_model.price).map_err(|error| {
        anyhow::anyhow!(
            "Failed to parse price '{}' of payment request for order '{}': {}",
            avro_model.price,
            avro_model.order_id,
            error,
        )
    })?;

    Ok(ProcessPaymentCommand::new(
        parse_uuid("saga_id", &avro_model.saga_id)?,
        parse_uuid("order_id", &avro_model.order_id)?,
        parse_uuid("customer_id", &avro_model.customer_id)?,
        price,
    ))
}

pub fn payment_response_from_payload(
    saga_id: Uuid,
    payload: PaymentResponsePayload,
) -> PaymentResponseAvroModel {
    PaymentResponseAvroModel {
        id: Uuid::new_v4().to_string(),
        saga_id: saga_id.to_string(),
        payment_id: payload.payment_id.to_string(),
        order_id: payload.order_id.to_string(),
        customer_id: payload.customer_id.to_string(),
        price: payload.price,
        payment_status: payload.payment_status,
        failure_messages: payload.failure_messages,
        created_at: payload.created_at,
    }
}

pub fn payment_created_event_to_avro(event: &PaymentCreatedEvent) -> PaymentCreatedEventAvroModel {
    PaymentCreatedEventAvroModel {
        id: event.id().to_string(),
        payment: payment_to_avro(event.payment()),
        created_at: event.created_at().to_string(),
    }
}

pub fn payment_updated_event_to_avro(event: &PaymentUpdatedEvent) -> PaymentUpdatedEventAvroModel {
    PaymentUpdatedEventAvroModel {
        id: event.id().to_string(),
        payment: payment_to_avro(event.payment()),
        created_at: event.created_at().to_string(),
    }
}

fn payment_to_avro(payment: &Payment) -> PaymentAvroModel {
    PaymentAvroModel {
        id: payment.id().to_string(),
        order_id: payment.order_id().to_string(),
        customer_id: payment.customer_id().to_string(),
        price: payment.price().to_string(),
        payment_status: payment.payment_status().as_str().to_string(),
    }
}

fn parse_uuid(field: &str, value: &str) -> anyhow::Result<Uuid> {
    Uuid::parse_str(value).map_err(|error| {
        anyhow::anyhow!(
            "Failed to parse {} from Avro model value '{}': {}",
            field,
            value,
            error,
        )
    })
}
