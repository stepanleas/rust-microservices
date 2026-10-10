use crate::api::api_products::__path_create;
use crate::api::api_products::__path_destroy;
use crate::api::api_products::__path_find_one;
use crate::api::api_products::__path_list_all;
use crate::api::api_products::__path_update;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "Products", description = "Product management endpoints.")
    ),
    paths(
        list_all,
        find_one,
        create,
        update,
        destroy,
    )
)]
pub(crate) struct ProductApiDoc;
