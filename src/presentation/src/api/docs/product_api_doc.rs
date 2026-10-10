use crate::api::api_products::{__path_find_by_id, __path_list};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "Products", description = "Products replicated from the catalog with the restaurant stock.")
    ),
    paths(list, find_by_id)
)]
pub(crate) struct ProductApiDoc;
