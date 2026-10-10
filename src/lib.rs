//! Code shared by the microservices.
//!
//! The typed ids and [`error::AppError`] are always available; everything that pulls in a
//! framework sits behind a cargo feature, so each crate of a service enables only what its
//! layer may depend on (e.g. the domain crate enables no feature at all).

pub mod domain;
pub mod error;

#[cfg(feature = "contracts")]
pub mod contracts;
#[cfg(feature = "db")]
pub mod db;
#[cfg(feature = "kafka")]
pub mod kafka;
#[cfg(feature = "outbox")]
pub mod outbox;
#[cfg(feature = "web")]
pub mod web;
