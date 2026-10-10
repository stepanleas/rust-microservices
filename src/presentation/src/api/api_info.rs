use crate::dto::responses::{AppInfoResponse, DataResponse};
use crate::state::AppState;
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get};
use serde_json::json;
use shared::web::{ApiError, ErrorResponse};

const APP: &str = "App";

/// Application info
///
/// Returns the environment the service runs in.
#[utoipa::path(
    tag = APP,
    operation_id = "app_info",
    responses(
        (status = 200, description = "Display application info", body = DataResponse<AppInfoResponse>),
        (status = 500, description = "Unexpected server error", body = ErrorResponse),
    )
)]
#[get("")]
pub async fn info(state: Data<AppState>) -> Result<impl Responder, ApiError> {
    Ok(HttpResponse::Ok().json(json!({ "data": AppInfoResponse::new(&state.environment) })))
}
