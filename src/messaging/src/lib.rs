//! Kafka adapters of the order service: the consumer side (one [`handlers::OrderMessageHandler`]
//! for every consumed topic) and the outbox publisher.

pub mod handlers;
#[cfg(test)]
mod handlers_tests;
pub mod mappers;
pub mod publishers;
#[cfg(test)]
mod publishers_tests;
