use shared::web::health::{__path_live, __path_ready, __path_startup};
use utoipa::OpenApi;

/// The probes are shared by every service (`shared::web::health`).
#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "Health", description = "Order healthcheck management endpoints.")
    ),
    paths(startup, live, ready)
)]
pub(crate) struct HealthCheckApiDoc;
