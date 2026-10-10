use crate::dto::requests::CreateOrderRequest;
use crate::dto::responses::{DataResponse, OrderResponse, OrderTrackingResponse};
use crate::state::AppState;
use actix_web::web::{Data, Path};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, get, post};
use application::commands::CreateOrderCommand;
use application::queries::TrackOrderQuery;
use serde_json::json;
use shared::web::{ApiError, ErrorResponse, ValidatedJson, blocking};
use uuid::Uuid;

const ORDERS: &str = "Orders";

/// Place an order
///
/// Validates the customer, the products and the prices, stores the order as `PENDING` and starts
/// the order saga. The restaurant approval request is written to the outbox in the same
/// transaction and relayed to Kafka asynchronously, so the response only contains the tracking id;
/// poll `GET /api/orders/{tracking_id}` to follow the order to `PAID` or `CANCELLED`.
#[tracing::instrument(skip(req, state))]
#[utoipa::path(
    tag = ORDERS,
    operation_id = "create_order",
    request_body = CreateOrderRequest,
    responses(
        (status = 201, description = "Order placed and saga started", body = DataResponse<OrderResponse>),
        (status = 400, description = "Malformed body or validation error", body = ErrorResponse,
            example = json!({ "code": 400, "message": "Validation error", "errors": [{ "field": "price", "message": "Price must be greater than 0" }] })),
        (status = 422, description = "Unknown customer or product, insufficient quantity, or prices that do not add up", body = ErrorResponse,
            example = json!({ "code": 422, "message": "Total price: $20.00 is not equal to order items total: $60.00!" })),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    ),
)]
#[post("")]
pub async fn create(
    req: HttpRequest,
    state: Data<AppState>,
    request: ValidatedJson<CreateOrderRequest>,
) -> Result<impl Responder, ApiError> {
    let correlation_id = req
        .extensions()
        .get::<String>()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    tracing::info!(%correlation_id, "Handling order create");

    let command = CreateOrderCommand::try_from(request.into_inner())?;
    let use_case = state.create_order.clone();
    let tracking_id = blocking(move || use_case.execute(command)).await?;

    Ok(HttpResponse::Created().json(json!({ "data": OrderResponse::new(tracking_id) })))
}

/// Track an order
///
/// Returns the current state of the order. `status` moves from `PENDING` to `APPROVED` once the
/// restaurant accepts it and to `PAID` once the payment completes; it becomes `CANCELLED` (with
/// `failure_messages`) when the restaurant rejects it or the payment fails.
#[tracing::instrument(skip(state))]
#[utoipa::path(
    tag = ORDERS,
    operation_id = "track_order",
    params(
        ("tracking_id" = Uuid, Path, description = "Tracking id returned when the order was placed"),
    ),
    responses(
        (status = 200, description = "Order found", body = DataResponse<OrderTrackingResponse>),
        (status = 400, description = "Tracking id is not a valid UUID", body = ErrorResponse),
        (status = 404, description = "No order with this tracking id", body = ErrorResponse,
            example = json!({ "code": 404, "message": "Could not find order by tracking id: 7c9e6679-7425-40de-944b-e07fc1f90ae7" })),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    ),
)]
#[get("/{tracking_id}")]
pub async fn track(
    state: Data<AppState>,
    tracking_id: Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let query = TrackOrderQuery::new(tracking_id.into_inner());
    let use_case = state.track_order.clone();
    let order = blocking(move || use_case.execute(query)).await?;

    Ok(HttpResponse::Ok().json(json!({ "data": OrderTrackingResponse::from(order) })))
}
