//! Kafka adapter: consumes `payment-request` and publishes the payment events and the
//! outbox messages (`payment-response`) with the contracts of the shared crate.

pub mod handlers;
#[cfg(test)]
mod handlers_tests;
pub mod mappers;
#[cfg(test)]
mod mappers_tests;
pub mod publishers;
#[cfg(test)]
mod publishers_tests;
