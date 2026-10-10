use crate::api::api_info::__path_info;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "App", description = "Restaurant info endpoints.")
    ),
    paths(info)
)]
pub(crate) struct AppInfoApiDoc;
