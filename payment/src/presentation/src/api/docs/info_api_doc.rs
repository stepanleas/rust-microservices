use crate::api::api_info::__path_info;
use crate::dto::responses::{AppInfoEnvelope, AppInfoResponse};
use shared::web::ErrorResponse;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "App", description = "Payment info management endpoints.")
    ),
    paths(
        info
    ),
    components(schemas(AppInfoEnvelope, AppInfoResponse, ErrorResponse))
)]
pub(crate) struct AppInfoApiDoc;
