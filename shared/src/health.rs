//! Readiness checks of a service's dependencies, independent of the web framework.
//!
//! The adapters (e.g. [`crate::db::DbHealthCheck`]) implement [`HealthCheck`]; the actix
//! handlers in `web::health` run them to answer the readiness probe.

/// One dependency the service needs to serve requests. Blocking: run it off the async workers.
pub trait HealthCheck: Send + Sync {
    /// Name shown in the readiness response, e.g. `database`.
    fn name(&self) -> &'static str;

    /// `Err` with a reason when the dependency is not usable right now.
    fn check(&self) -> Result<(), String>;
}
