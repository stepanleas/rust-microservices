use crate::api::api_categories::__path_create;
use crate::api::api_categories::__path_destroy;
use crate::api::api_categories::__path_find_one;
use crate::api::api_categories::__path_list_all;
use crate::api::api_categories::__path_update;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "Categories", description = "Category management endpoints.")
    ),
    paths(
        list_all,
        find_one,
        create,
        update,
        destroy,
    )
)]
pub(crate) struct CategoryApiDoc;
