use crate::api::api_customer::__path_create;
use crate::api::api_customer::__path_update;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "Customers", description = "Customer management endpoints.")
    ),
    paths(
        create,
        update,
    )
)]
pub(crate) struct CustomerApiDoc;
