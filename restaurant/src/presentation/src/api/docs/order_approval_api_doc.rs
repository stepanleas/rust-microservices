use crate::api::api_order_approvals::__path_find_by_order_id;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "Order approvals", description = "Restaurant decisions taken during the order saga.")
    ),
    paths(find_by_order_id)
)]
pub(crate) struct OrderApprovalApiDoc;
