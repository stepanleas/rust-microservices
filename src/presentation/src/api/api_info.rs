use crate::dto::responses::{AppInfoDataResponse, AppInfoResponse};
use crate::state::AppState;
use actix_web::{HttpResponse, Responder, get, web};
use shared::web::{ApiError, ErrorResponse};

const APP: &str = "App";

/// Application info.
///
/// Returns runtime information about the restaurant service.
#[utoipa::path(
    tag = APP,
    operation_id = "app_info",
    responses(
        (status = 200, description = "Application info", body = AppInfoDataResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[get("")]
pub async fn info(state: web::Data<AppState>) -> Result<impl Responder, ApiError> {
    Ok(HttpResponse::Ok().json(AppInfoDataResponse {
        data: AppInfoResponse::new(&state.environment),
    }))
}
