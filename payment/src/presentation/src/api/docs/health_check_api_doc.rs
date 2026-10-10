use crate::api::api_health_check::__path_live;
use crate::api::api_health_check::__path_ready;
use crate::api::api_health_check::__path_startup;
use crate::dto::responses::{HealthCheckEnvelope, HealthCheckResponse};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "Health", description = "Payment healthcheck management endpoints.")
    ),
    paths(
        startup,
        live,
        ready,
    ),
    components(schemas(HealthCheckEnvelope, HealthCheckResponse))
)]
pub(crate) struct HealthCheckApiDoc;
