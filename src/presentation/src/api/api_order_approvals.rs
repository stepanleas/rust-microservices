use crate::dto::responses::{OrderApprovalDataResponse, OrderApprovalResponse};
use crate::state::AppState;
use actix_web::web::Path;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, get, web};
use shared::web::{ApiError, ErrorResponse, blocking};
use uuid::Uuid;

const ORDER_APPROVALS: &str = "Order approvals";

/// Get the restaurant approval of an order.
///
/// Returns the decision the restaurant took for the order while processing the
/// order saga: `APPROVED`, `REJECTED` (with the failure reasons) or `CANCELLED`
/// (approved first, then cancelled by the saga compensation and the stock given back).
#[tracing::instrument(skip(req, state))]
#[utoipa::path(
    tag = ORDER_APPROVALS,
    operation_id = "find_order_approval_by_order_id",
    params(
        ("order_id" = Uuid, Path, description = "Id of the order (as created by the order service)")
    ),
    responses(
        (status = 200, description = "Order approval found", body = OrderApprovalDataResponse),
        (status = 400, description = "Invalid order id", body = ErrorResponse),
        (status = 404, description = "No approval exists for the order yet", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
)]
#[get("/{order_id}")]
pub async fn find_by_order_id(
    req: HttpRequest,
    state: web::Data<AppState>,
    order_id: Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    {
        let extensions = req.extensions();
        let correlation_id = extensions
            .get::<String>()
            .map(String::as_str)
            .unwrap_or("unknown");

        tracing::info!(%correlation_id, "Handling order approval lookup");
    }

    let use_case = state.get_order_approval.clone();
    let order_id = order_id.into_inner();
    let approval = blocking(move || use_case.execute(order_id)).await?;

    Ok(HttpResponse::Ok().json(OrderApprovalDataResponse {
        data: OrderApprovalResponse::from(approval),
    }))
}
