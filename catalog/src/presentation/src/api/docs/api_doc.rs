use crate::api::docs::category_api_doc::CategoryApiDoc;
use crate::api::docs::health_check_api_doc::HealthCheckApiDoc;
use crate::api::docs::info_api_doc::AppInfoApiDoc;
use crate::api::docs::product_api_doc::ProductApiDoc;
use utoipa::OpenApi;
use utoipa::openapi::OpenApi as OpenApiStruct;

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/api/products", api = ProductApiDoc),
        (path = "/api/categories", api = CategoryApiDoc),
        (path = "/api/health", api = HealthCheckApiDoc),
        (path = "/api/info", api = AppInfoApiDoc),
    )
)]
pub struct ApiDoc;

pub fn open_api_docs() -> OpenApiStruct {
    ApiDoc::openapi()
}
