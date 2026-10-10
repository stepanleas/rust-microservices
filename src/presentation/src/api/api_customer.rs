use crate::api::correlation_id;
use crate::dto::requests::{CreateCustomerRequest, UpdateCustomerRequest};
use crate::dto::responses::{CustomerResponse, DataResponse};
use crate::state::AppState;
use actix_web::web::Path;
use actix_web::{HttpRequest, HttpResponse, Responder, post, put, web};
use application::commands::{CreateCustomerCommand, UpdateCustomerCommand};
use serde_json::json;
use shared::web::{ApiError, ErrorResponse, ValidatedJson, blocking};
use uuid::Uuid;

const CUSTOMERS: &str = "Customers";

/// Create a customer
///
/// Persists the customer and publishes a `customer-created` event to Kafka so the
/// order service can keep its local customer replica in sync.
#[tracing::instrument(skip(req, state))]
#[utoipa::path(
    tag = CUSTOMERS,
    operation_id = "create_customer",
    responses(
        (status = 201, description = "Customer created", body = DataResponse<CustomerResponse>),
        (status = 400, description = "Malformed JSON or request validation failed", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    request_body = CreateCustomerRequest,
)]
#[post("")]
pub async fn create(
    req: HttpRequest,
    state: web::Data<AppState>,
    request: ValidatedJson<CreateCustomerRequest>,
) -> Result<impl Responder, ApiError> {
    let correlation_id = correlation_id(&req);
    tracing::info!(%correlation_id, "Handling customer create");

    let payload = request.into_inner();
    let command = CreateCustomerCommand::new(
        payload.user_name.clone(),
        payload.first_name.clone(),
        payload.last_name.clone(),
    );

    let use_case = state.create_customer.clone();
    let customer = blocking(move || use_case.execute(command)).await?;

    Ok(HttpResponse::Created().json(json!({ "data": CustomerResponse::from(customer) })))
}

/// Update a customer
///
/// Upserts the customer by id and publishes a `customer-updated` event to Kafka.
#[tracing::instrument(skip(req, state))]
#[utoipa::path(
    tag = CUSTOMERS,
    operation_id = "update_customer",
    responses(
        (status = 200, description = "Customer updated", body = DataResponse<CustomerResponse>),
        (status = 400, description = "Malformed JSON or request validation failed", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    params(
        ("id" = Uuid, Path, description = "Id of the customer to update")
    ),
    request_body = UpdateCustomerRequest,
)]
#[put("/{id}")]
pub async fn update(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: Path<Uuid>,
    request: ValidatedJson<UpdateCustomerRequest>,
) -> Result<impl Responder, ApiError> {
    let correlation_id = correlation_id(&req);
    tracing::info!(%correlation_id, "Handling customer update");

    let payload = request.into_inner();
    let command = UpdateCustomerCommand::new(
        id.into_inner(),
        payload.user_name.clone(),
        payload.first_name.clone(),
        payload.last_name.clone(),
    );

    let use_case = state.update_customer.clone();
    let customer = blocking(move || use_case.execute(command)).await?;

    Ok(HttpResponse::Ok().json(json!({ "data": CustomerResponse::from(customer) })))
}
