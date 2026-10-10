use crate::dto::responses::{ProductDataResponse, ProductListDataResponse, ProductResponse};
use crate::state::AppState;
use actix_web::web::Path;
use actix_web::{HttpResponse, Responder, get, web};
use shared::web::{ApiError, ErrorResponse, blocking};
use uuid::Uuid;

const PRODUCTS: &str = "Products";

/// List restaurant products.
///
/// Returns the products replicated from the catalog service together with the
/// stock the restaurant has left.
#[utoipa::path(
    tag = PRODUCTS,
    operation_id = "list_products",
    responses(
        (status = 200, description = "List of products", body = ProductListDataResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
)]
#[get("")]
pub async fn list(state: web::Data<AppState>) -> Result<impl Responder, ApiError> {
    let use_case = state.list_products.clone();
    let products = blocking(move || use_case.execute()).await?;

    Ok(HttpResponse::Ok().json(ProductListDataResponse {
        data: products.into_iter().map(ProductResponse::from).collect(),
    }))
}

/// Get a restaurant product.
///
/// Returns a single product with its remaining stock.
#[utoipa::path(
    tag = PRODUCTS,
    operation_id = "find_product_by_id",
    params(
        ("id" = Uuid, Path, description = "Id of the product")
    ),
    responses(
        (status = 200, description = "Product found", body = ProductDataResponse),
        (status = 400, description = "Invalid product id", body = ErrorResponse),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
)]
#[get("/{id}")]
pub async fn find_by_id(
    state: web::Data<AppState>,
    id: Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let use_case = state.get_product.clone();
    let product_id = id.into_inner();
    let product = blocking(move || use_case.execute(product_id)).await?;

    Ok(HttpResponse::Ok().json(ProductDataResponse {
        data: ProductResponse::from(product),
    }))
}
