use crate::api::api_orders::__path_create;
use crate::api::api_orders::__path_track;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "Orders", description = "Place orders and track them through the order saga.")
    ),
    paths(
        create,
        track,
    )
)]
pub(crate) struct OrderApiDoc;
