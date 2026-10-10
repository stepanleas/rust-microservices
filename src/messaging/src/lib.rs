//! Kafka adapter of the restaurant service: the consumer side (one `MessageHandler` for every
//! topic the service listens to) and the outbox publisher.

pub mod handlers;
#[cfg(test)]
mod handlers_tests;
pub mod mappers;
#[cfg(test)]
mod mappers_tests;
pub mod publishers;
#[cfg(test)]
mod publishers_tests;
