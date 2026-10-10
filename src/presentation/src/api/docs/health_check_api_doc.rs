use crate::api::api_health_check::{__path_live, __path_ready, __path_startup};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "Health", description = "Restaurant healthcheck endpoints.")
    ),
    paths(startup, live, ready)
)]
pub(crate) struct HealthCheckApiDoc;
