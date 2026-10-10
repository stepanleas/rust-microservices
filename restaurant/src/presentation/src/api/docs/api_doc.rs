use crate::api::docs::health_check_api_doc::HealthCheckApiDoc;
use crate::api::docs::info_api_doc::AppInfoApiDoc;
use crate::api::docs::order_approval_api_doc::OrderApprovalApiDoc;
use crate::api::docs::product_api_doc::ProductApiDoc;
use utoipa::OpenApi;
use utoipa::openapi::OpenApi as OpenApiStruct;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Restaurant API",
        description = "Restaurant service of the order saga. Approves or rejects orders \
            received on `restaurant-approval-request` and answers on \
            `restaurant-approval-response`, and gives the stock of an approved order back \
            when the saga compensates it (`restaurant-approval-cancel-request`). \
            The HTTP API is read-only."
    ),
    nest(
        (path = "/api/order-approvals", api = OrderApprovalApiDoc),
        (path = "/api/products", api = ProductApiDoc),
        (path = "/api/health", api = HealthCheckApiDoc),
        (path = "/api/info", api = AppInfoApiDoc),
    )
)]
pub struct ApiDoc;

pub fn open_api_docs() -> OpenApiStruct {
    ApiDoc::openapi()
}
