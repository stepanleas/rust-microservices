use crate::api::correlation_id;
use crate::dto::requests::{CreateProductRequest, UpdateProductRequest};
use crate::dto::responses::{DataResponse, ProductResponse};
use crate::state::AppState;
use actix_web::web::Path;
use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, post, put, web};
use application::commands::{CreateProductCommand, DeleteProductCommand, UpdateProductCommand};
use application::queries::FindProductQuery;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde_json::json;
use shared::web::{ApiError, ErrorResponse, ValidatedJson, blocking};
use uuid::Uuid;

const PRODUCTS: &str = "Products";

fn to_decimal(price: f64) -> Result<Decimal, ApiError> {
    Decimal::from_f64(price).ok_or_else(|| ApiError::bad_request("Invalid price value"))
}

/// List products
///
/// Returns every product in the catalog together with its category.
#[utoipa::path(
    tag = PRODUCTS,
    operation_id = "list_all_products",
    responses(
        (status = 200, description = "List of products", body = DataResponse<Vec<ProductResponse>>),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[get("")]
pub async fn list_all(state: web::Data<AppState>) -> Result<impl Responder, ApiError> {
    let use_case = state.list_products.clone();
    let products = blocking(move || use_case.execute()).await?;

    let data = products
        .into_iter()
        .map(ProductResponse::from)
        .collect::<Vec<ProductResponse>>();

    Ok(HttpResponse::Ok().json(json!({ "data": data })))
}

/// Get a product
#[utoipa::path(
    tag = PRODUCTS,
    operation_id = "find_product_by_id",
    responses(
        (status = 200, description = "The product", body = DataResponse<ProductResponse>),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    params(
        ("id" = Uuid, Path, description = "Id of the product item")
    ),
)]
#[get("/{id}")]
pub async fn find_one(
    state: web::Data<AppState>,
    id: Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let use_case = state.find_product.clone();
    let query = FindProductQuery::new(id.into_inner());
    let product = blocking(move || use_case.execute(query)).await?;

    Ok(HttpResponse::Ok().json(json!({ "data": ProductResponse::from(product) })))
}

/// Create a product
///
/// Persists the product and publishes a `product-created` event to Kafka.
#[tracing::instrument(skip(req, state))]
#[utoipa::path(
    tag = PRODUCTS,
    operation_id = "create_product",
    responses(
        (status = 201, description = "Product created", body = DataResponse<ProductResponse>),
        (status = 400, description = "Malformed JSON or request validation failed", body = ErrorResponse),
        (status = 404, description = "Category not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    request_body = CreateProductRequest,
)]
#[post("")]
pub async fn create(
    req: HttpRequest,
    state: web::Data<AppState>,
    request: ValidatedJson<CreateProductRequest>,
) -> Result<impl Responder, ApiError> {
    let correlation_id = correlation_id(&req);
    tracing::info!(%correlation_id, "Handling product create");

    let payload = request.into_inner();
    let command = CreateProductCommand::new(
        payload.title.clone(),
        payload.description.clone(),
        payload.quantity,
        to_decimal(payload.price)?,
        payload.category_id,
    );

    let use_case = state.create_product.clone();
    let product = blocking(move || use_case.execute(command)).await?;

    Ok(HttpResponse::Created().json(json!({ "data": ProductResponse::from(product) })))
}

/// Update a product
///
/// Updates the product and publishes a `product-updated` event to Kafka.
#[tracing::instrument(skip(req, state))]
#[utoipa::path(
    tag = PRODUCTS,
    operation_id = "update_product",
    responses(
        (status = 200, description = "Product updated", body = DataResponse<ProductResponse>),
        (status = 400, description = "Malformed JSON or request validation failed", body = ErrorResponse),
        (status = 404, description = "Product or category not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    params(
        ("id" = Uuid, Path, description = "Id of the product item to update")
    ),
    request_body = UpdateProductRequest,
)]
#[put("/{id}")]
pub async fn update(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: Path<Uuid>,
    request: ValidatedJson<UpdateProductRequest>,
) -> Result<impl Responder, ApiError> {
    let correlation_id = correlation_id(&req);
    tracing::info!(%correlation_id, "Handling product update");

    let payload = request.into_inner();
    let command = UpdateProductCommand::new(
        id.into_inner(),
        payload.title.clone(),
        payload.description.clone(),
        payload.quantity,
        to_decimal(payload.price)?,
        payload.category_id,
    );

    let use_case = state.update_product.clone();
    let product = blocking(move || use_case.execute(command)).await?;

    Ok(HttpResponse::Ok().json(json!({ "data": ProductResponse::from(product) })))
}

/// Delete a product
///
/// Deletes the product and publishes a `product-deleted` event to Kafka.
#[tracing::instrument(skip(req, state))]
#[utoipa::path(
    tag = PRODUCTS,
    operation_id = "delete_product",
    responses(
        (status = 204, description = "Product deleted"),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    params(
        ("id" = Uuid, Path, description = "Id of the product item to delete")
    )
)]
#[delete("/{id}")]
pub async fn destroy(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let correlation_id = correlation_id(&req);
    tracing::info!(%correlation_id, "Handling product delete");

    let use_case = state.delete_product.clone();
    let command = DeleteProductCommand::new(id.into_inner());
    blocking(move || use_case.execute(command)).await?;

    Ok(HttpResponse::NoContent().finish())
}
