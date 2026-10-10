use crate::api::api_health_check::__path_live;
use crate::api::api_health_check::__path_ready;
use crate::api::api_health_check::__path_startup;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "Health", description = "Order healthcheck management endpoints.")
    ),
    paths(
        startup,
        live,
        ready,
    )
)]
pub(crate) struct HealthCheckApiDoc;
