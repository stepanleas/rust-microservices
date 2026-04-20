use crate::app_state::AppState;
use crate::error::ApiError;
use crate::requests::product_requests::{CreateProductRequest, UpdateProductRequest};
use crate::responses::product_responses::ProductResponse;
use crate::validation::ValidatedJson;
use actix_web::web::Path;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, delete, get, post, put, web};
use anyhow::anyhow;
use application::product::commands::{
    CreateProductCommand, DeleteProductCommand, UpdateProductCommand,
};
use application::product::handlers::{
    CreateProductCommandHandler, DeleteProductCommandHandler, FindProductQueryHandler,
    ListAllProductQueryHandler, UpdateProductCommandHandler,
};
use application::product::queries::FindProductQuery;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde_json::json;
use uuid::Uuid;

const PRODUCTS: &str = "Products";

#[utoipa::path(
    tag = PRODUCTS,
    operation_id = "list_all_products",
    responses(
        (status = 200, description = "List current product items", body = [ProductResponse])
    )
)]
#[get("")]
pub async fn list_all(req: HttpRequest) -> Result<impl Responder, ApiError> {
    let state = req
        .app_data::<web::Data<AppState>>()
        .ok_or_else(|| ApiError::internal(anyhow!("Missing app state")))?;

    let handler = ListAllProductQueryHandler::new(state.product_repository.clone());

    let categories = handler.execute().await?;
    let data = categories
        .into_iter()
        .map(ProductResponse::from)
        .collect::<Vec<ProductResponse>>();

    Ok(HttpResponse::Ok().json(json!({ "data": data })))
}

#[utoipa::path(
    tag = PRODUCTS,
    operation_id = "find_product_by_id",
    responses(
        (status = 200, description = "Get product item by id", body = [ProductResponse])
    ),
    params(
        ("id" = Uuid, Path, description = "Id of the product item")
    ),
)]
#[get("/{id}")]
pub async fn find_one(req: HttpRequest, id: Path<Uuid>) -> Result<impl Responder, ApiError> {
    let state = req
        .app_data::<web::Data<AppState>>()
        .ok_or_else(|| ApiError::internal(anyhow!("Missing app state")))?;

    let handler = FindProductQueryHandler::new(state.product_repository.clone());

    let product = handler
        .execute(FindProductQuery::new(Some(id.into_inner())))
        .await?;

    Ok(HttpResponse::Ok().json(json!({ "data": ProductResponse::from(product) })))
}

#[tracing::instrument(skip(req))]
#[utoipa::path(
    tag = PRODUCTS,
    operation_id = "create_product",
    responses(
        (status = 201, description = "Create a product item", body = [ProductResponse])
    ),
    request_body = CreateProductRequest,
)]
#[post("")]
pub async fn create(
    req: HttpRequest,
    request: ValidatedJson<CreateProductRequest>,
) -> Result<impl Responder, ApiError> {
    let correlation_id = req
        .extensions()
        .get::<String>()
        .cloned()
        .unwrap_or("unknown".to_string());

    tracing::info!(%correlation_id, "Handling product create");

    let payload = request.into_inner();

    let state = req
        .app_data::<web::Data<AppState>>()
        .ok_or_else(|| ApiError::internal(anyhow!("Missing app state")))?;

    let handler = CreateProductCommandHandler::new(
        state.product_repository.clone(),
        state.category_repository.clone(),
        state.product_message_publisher.clone(),
    );

    let command = CreateProductCommand::new(
        &payload.title,
        &payload.description,
        payload.quantity,
        Decimal::from_f64(payload.price).expect("Invalid price value"),
        payload.category_id,
    );

    let product = handler.execute(command).await?;

    Ok(HttpResponse::Created().json(json!({ "data": ProductResponse::from(product) })))
}

#[tracing::instrument(skip(req))]
#[utoipa::path(
    tag = PRODUCTS,
    operation_id = "update_product",
    responses(
        (status = 200, description = "Update a product item", body = [ProductResponse])
    ),
    params(
        ("id", description = "Id of the product item to update")
    ),
    request_body = UpdateProductRequest,
)]
#[put("/{id}")]
pub async fn update(
    req: HttpRequest,
    id: Path<Uuid>,
    request: ValidatedJson<UpdateProductRequest>,
) -> Result<impl Responder, ApiError> {
    let correlation_id = req
        .extensions()
        .get::<String>()
        .cloned()
        .unwrap_or("unknown".to_string());

    tracing::info!(%correlation_id, "Handling product update");

    let payload = request.into_inner();

    let state = req
        .app_data::<web::Data<AppState>>()
        .ok_or_else(|| ApiError::internal(anyhow!("Missing app state")))?;

    let handler = UpdateProductCommandHandler::new(
        state.product_repository.clone(),
        state.category_repository.clone(),
        state.product_message_publisher.clone(),
    );

    let command = UpdateProductCommand::new(
        id.into_inner(),
        &payload.title,
        &payload.description,
        payload.quantity,
        Decimal::from_f64(payload.price).expect("Invalid price value"),
        payload.category_id,
    );

    let product = handler.execute(command).await?;

    Ok(HttpResponse::Created().json(json!({ "data": ProductResponse::from(product) })))
}

#[tracing::instrument(skip(req))]
#[utoipa::path(
    tag = PRODUCTS,
    operation_id = "delete_product",
    responses(
        (status = 204, description = "Delete a product item", body = [ProductResponse])
    ),
    params(
        ("id", description = "Id of the product item to delete")
    )
)]
#[delete("/{id}")]
pub async fn delete(req: HttpRequest, id: Path<Uuid>) -> Result<impl Responder, ApiError> {
    let correlation_id = req
        .extensions()
        .get::<String>()
        .cloned()
        .unwrap_or("unknown".to_string());

    tracing::info!(%correlation_id, "Handling product delete");

    let state = req
        .app_data::<web::Data<AppState>>()
        .ok_or_else(|| ApiError::internal(anyhow!("Missing app state")))?;

    let handler = DeleteProductCommandHandler::new(
        state.product_repository.clone(),
        state.product_message_publisher.clone(),
    );
    let command = DeleteProductCommand::new(id.into_inner());

    handler.execute(command).await?;

    Ok(HttpResponse::NoContent().finish())
}
