use crate::api::docs::customer_api_doc::CustomerApiDoc;
use crate::api::docs::health_check_api_doc::HealthCheckApiDoc;
use crate::api::docs::info_api_doc::AppInfoApiDoc;
use utoipa::OpenApi;
use utoipa::openapi::OpenApi as OpenApiStruct;

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/api/customers", api = CustomerApiDoc),
        (path = "/api/health", api = HealthCheckApiDoc),
        (path = "/api/info", api = AppInfoApiDoc),
    )
)]
pub struct ApiDoc;

pub fn open_api_docs() -> OpenApiStruct {
    ApiDoc::openapi()
}
