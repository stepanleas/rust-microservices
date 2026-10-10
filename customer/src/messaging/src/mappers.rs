//! Domain events -> wire contracts (`shared::contracts::customer`).

use domain::aggregates::customer::Customer;
use domain::events::{CustomerCreatedEvent, CustomerUpdatedEvent};
use shared::contracts::customer::{
    CustomerAvroModel, CustomerCreatedEventAvroModel, CustomerUpdatedEventAvroModel,
};

fn customer_to_avro(customer: &Customer) -> CustomerAvroModel {
    CustomerAvroModel {
        id: customer.id().to_string(),
        user_name: customer.user_name().to_string(),
        first_name: customer.first_name().to_string(),
        last_name: customer.last_name().to_string(),
    }
}

pub fn customer_created_to_avro(event: &CustomerCreatedEvent) -> CustomerCreatedEventAvroModel {
    CustomerCreatedEventAvroModel {
        id: event.id().to_string(),
        customer: customer_to_avro(event.customer()),
        created_at: event.created_at().to_string(),
    }
}

pub fn customer_updated_to_avro(event: &CustomerUpdatedEvent) -> CustomerUpdatedEventAvroModel {
    CustomerUpdatedEventAvroModel {
        id: event.id().to_string(),
        customer: customer_to_avro(event.customer()),
        created_at: event.created_at().to_string(),
    }
}

#[cfg(test)]
#[path = "mappers_tests.rs"]
mod tests;
