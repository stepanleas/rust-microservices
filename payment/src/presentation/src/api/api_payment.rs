use crate::dto::requests::{CreatePaymentRequest, UpdatePaymentRequest};
use crate::dto::responses::{PaymentEnvelope, PaymentResponse};
use crate::state::AppState;
use actix_web::web::Path;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, get, post, put, web};
use application::commands::{CreatePaymentCommand, UpdatePaymentCommand};
use application::queries::{FindPaymentByIdQuery, FindPaymentByOrderIdQuery};
use domain::enums::PaymentStatus;
use rust_decimal::Decimal;
use shared::web::{ApiError, ErrorResponse, ValidatedJson, blocking};
use uuid::Uuid;

const PAYMENTS: &str = "Payments";

fn log_correlation_id(req: &HttpRequest, action: &str) {
    let extensions = req.extensions();
    let correlation_id = extensions
        .get::<String>()
        .map(String::as_str)
        .unwrap_or("unknown");

    tracing::info!(%correlation_id, "Handling {}", action);
}

fn ok(payment: PaymentResponse) -> HttpResponse {
    HttpResponse::Ok().json(PaymentEnvelope { data: payment })
}

/// Get a payment by id.
///
/// Returns a payment either created through the REST API or by the order saga
/// (`payment-request` topic).
#[tracing::instrument(skip(req, state))]
#[utoipa::path(
    tag = PAYMENTS,
    operation_id = "get_payment",
    params(
        ("id" = Uuid, Path, description = "Id of the payment")
    ),
    responses(
        (status = 200, description = "Payment found", body = PaymentEnvelope),
        (status = 400, description = "Malformed payment id", body = ErrorResponse),
        (status = 404, description = "Payment not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
)]
#[get("/{id}")]
pub async fn get(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    log_correlation_id(&req, "payment get");

    let use_case = state.find_payment_by_id.clone();
    let query = FindPaymentByIdQuery::new(id.into_inner());
    let payment = blocking(move || use_case.execute(query)).await?;

    Ok(ok(PaymentResponse::from(payment)))
}

/// Get the payment of an order.
///
/// Useful to follow the saga: once the order service sends a `payment-request`,
/// the payment for the order appears here with status `COMPLETED` or `FAILED`.
#[tracing::instrument(skip(req, state))]
#[utoipa::path(
    tag = PAYMENTS,
    operation_id = "get_payment_by_order_id",
    params(
        ("order_id" = Uuid, Path, description = "Id of the order the payment belongs to")
    ),
    responses(
        (status = 200, description = "Payment found", body = PaymentEnvelope),
        (status = 400, description = "Malformed order id", body = ErrorResponse),
        (status = 404, description = "No payment exists for the order", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
)]
#[get("/orders/{order_id}")]
pub async fn get_by_order_id(
    req: HttpRequest,
    state: web::Data<AppState>,
    order_id: Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    log_correlation_id(&req, "payment get by order id");

    let use_case = state.find_payment_by_order_id.clone();
    let query = FindPaymentByOrderIdQuery::new(order_id.into_inner());
    let payment = blocking(move || use_case.execute(query)).await?;

    Ok(ok(PaymentResponse::from(payment)))
}

/// Create a payment manually.
///
/// Creates a `COMPLETED` payment and publishes a `payment-created` event.
/// Saga payments are created from the `payment-request` topic instead.
#[tracing::instrument(skip(req, state))]
#[utoipa::path(
    tag = PAYMENTS,
    operation_id = "create_payment",
    request_body = CreatePaymentRequest,
    responses(
        (status = 201, description = "Payment created", body = PaymentEnvelope),
        (status = 400, description = "Invalid request body", body = ErrorResponse),
        (status = 409, description = "A payment already exists for the order", body = ErrorResponse),
        (status = 422, description = "Payment violates a business rule", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
)]
#[post("")]
pub async fn create(
    req: HttpRequest,
    state: web::Data<AppState>,
    request: ValidatedJson<CreatePaymentRequest>,
) -> Result<impl Responder, ApiError> {
    log_correlation_id(&req, "payment create");

    let payload = request.into_inner();
    let price = Decimal::try_from(payload.price)
        .map_err(|e| ApiError::bad_request(format!("Invalid price: {}", e)))?;

    let command = CreatePaymentCommand::new(payload.order_id, payload.customer_id, price);
    let use_case = state.create_payment.clone();
    let payment = blocking(move || use_case.execute(command)).await?;

    Ok(HttpResponse::Created().json(PaymentEnvelope {
        data: PaymentResponse::from(payment),
    }))
}

/// Update a payment.
///
/// Replaces order, customer, price and status of an existing payment and
/// publishes a `payment-updated` event.
#[tracing::instrument(skip(req, state))]
#[utoipa::path(
    tag = PAYMENTS,
    operation_id = "update_payment",
    params(
        ("id" = Uuid, Path, description = "Id of the payment to update")
    ),
    request_body = UpdatePaymentRequest,
    responses(
        (status = 200, description = "Payment updated", body = PaymentEnvelope),
        (status = 400, description = "Invalid request body or unknown payment status", body = ErrorResponse),
        (status = 404, description = "Payment not found", body = ErrorResponse),
        (status = 422, description = "Payment violates a business rule", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
)]
#[put("/{id}")]
pub async fn update(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: Path<Uuid>,
    request: ValidatedJson<UpdatePaymentRequest>,
) -> Result<impl Responder, ApiError> {
    log_correlation_id(&req, "payment update");

    let payload = request.into_inner();
    let price = Decimal::try_from(payload.price)
        .map_err(|e| ApiError::bad_request(format!("Invalid price: {}", e)))?;

    let payment_status = PaymentStatus::parse(&payload.payment_status).ok_or_else(|| {
        ApiError::bad_request(format!(
            "Invalid payment status: {}. Expected one of COMPLETED, FAILED, CANCELLED",
            payload.payment_status
        ))
    })?;

    let command = UpdatePaymentCommand::new(
        id.into_inner(),
        payload.order_id,
        payload.customer_id,
        price,
        payment_status,
    );

    let use_case = state.update_payment.clone();
    let payment = blocking(move || use_case.execute(command)).await?;

    Ok(ok(PaymentResponse::from(payment)))
}
