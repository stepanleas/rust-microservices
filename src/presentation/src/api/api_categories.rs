use crate::api::correlation_id;
use crate::dto::requests::{CreateCategoryRequest, UpdateCategoryRequest};
use crate::dto::responses::{CategoryResponse, DataResponse};
use crate::state::AppState;
use actix_web::web::Path;
use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, post, put, web};
use application::commands::{CreateCategoryCommand, DeleteCategoryCommand, UpdateCategoryCommand};
use application::queries::FindCategoryQuery;
use serde_json::json;
use shared::web::{ApiError, ErrorResponse, ValidatedJson, blocking};
use tracing::info;
use uuid::Uuid;

const CATEGORIES: &str = "Categories";

/// List categories
///
/// Returns every category stored in the catalog.
#[utoipa::path(
    tag = CATEGORIES,
    operation_id = "list_all_categories",
    responses(
        (status = 200, description = "List of categories", body = DataResponse<Vec<CategoryResponse>>),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[get("")]
pub async fn list_all(state: web::Data<AppState>) -> Result<impl Responder, ApiError> {
    let use_case = state.list_categories.clone();
    let categories = blocking(move || use_case.execute()).await?;

    let data = categories
        .into_iter()
        .map(CategoryResponse::from)
        .collect::<Vec<CategoryResponse>>();

    Ok(HttpResponse::Ok().json(json!({ "data": data })))
}

/// Get a category
///
/// Returns a single category by its id.
#[utoipa::path(
    tag = CATEGORIES,
    operation_id = "find_category_by_id",
    responses(
        (status = 200, description = "The category", body = DataResponse<CategoryResponse>),
        (status = 404, description = "Category not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    params(
        ("id" = Uuid, Path, description = "Id of the category item")
    ),
)]
#[get("/{id}")]
pub async fn find_one(
    state: web::Data<AppState>,
    id: Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let use_case = state.find_category.clone();
    let query = FindCategoryQuery::new(id.into_inner());
    let category = blocking(move || use_case.execute(query)).await?;

    Ok(HttpResponse::Ok().json(json!({ "data": CategoryResponse::from(category) })))
}

/// Create a category
#[utoipa::path(
    tag = CATEGORIES,
    operation_id = "create_category",
    responses(
        (status = 201, description = "Category created", body = DataResponse<CategoryResponse>),
        (status = 400, description = "Malformed JSON or request validation failed", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    request_body = CreateCategoryRequest,
)]
#[post("")]
pub async fn create(
    req: HttpRequest,
    state: web::Data<AppState>,
    request: ValidatedJson<CreateCategoryRequest>,
) -> Result<impl Responder, ApiError> {
    let correlation_id = correlation_id(&req);
    info!(%correlation_id, "Handling category create");

    let payload = request.into_inner();
    let command = CreateCategoryCommand::new(payload.title.clone(), payload.description.clone());

    let use_case = state.create_category.clone();
    let category = blocking(move || use_case.execute(command)).await?;

    Ok(HttpResponse::Created().json(json!({ "data": CategoryResponse::from(category) })))
}

/// Update a category
#[utoipa::path(
    tag = CATEGORIES,
    operation_id = "update_category",
    responses(
        (status = 200, description = "Category updated", body = DataResponse<CategoryResponse>),
        (status = 400, description = "Malformed JSON or request validation failed", body = ErrorResponse),
        (status = 404, description = "Category not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    params(
        ("id" = Uuid, Path, description = "Id of the category item to update")
    ),
    request_body = UpdateCategoryRequest,
)]
#[put("/{id}")]
pub async fn update(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: Path<Uuid>,
    request: ValidatedJson<UpdateCategoryRequest>,
) -> Result<impl Responder, ApiError> {
    let correlation_id = correlation_id(&req);
    info!(%correlation_id, "Handling category update");

    let payload = request.into_inner();
    let command = UpdateCategoryCommand::new(
        id.into_inner(),
        payload.title.clone(),
        payload.description.clone(),
    );

    let use_case = state.update_category.clone();
    let category = blocking(move || use_case.execute(command)).await?;

    Ok(HttpResponse::Ok().json(json!({ "data": CategoryResponse::from(category) })))
}

/// Delete a category
#[utoipa::path(
    tag = CATEGORIES,
    operation_id = "delete_category",
    responses(
        (status = 204, description = "Category deleted"),
        (status = 404, description = "Category not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    params(
        ("id" = Uuid, Path, description = "Id of the category item to delete")
    )
)]
#[delete("/{id}")]
pub async fn destroy(
    state: web::Data<AppState>,
    id: Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let use_case = state.delete_category.clone();
    let command = DeleteCategoryCommand::new(id.into_inner());
    blocking(move || use_case.execute(command)).await?;

    Ok(HttpResponse::NoContent().finish())
}
