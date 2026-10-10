//! actix-web building blocks shared by the services' presentation crates.

mod blocking;
mod correlation_id;
mod error;
pub mod health;
mod server;
mod validation;

pub use blocking::blocking;
pub use correlation_id::{CORRELATION_ID, CorrelationIdMiddleware};
pub use error::{ApiError, ErrorResponse};
pub use server::{WORKERS_STOP_TIMEOUT, serve};
pub use validation::{ValidatedJson, ValidationFieldError};
