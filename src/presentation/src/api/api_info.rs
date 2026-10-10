use crate::dto::responses::{AppInfoEnvelope, AppInfoResponse};
use crate::state::AppState;
use actix_web::{HttpResponse, Responder, get, web};
use shared::web::{ApiError, ErrorResponse};

const APP: &str = "App";

/// Application info.
///
/// Returns the environment the service is running in.
#[utoipa::path(
    tag = APP,
    operation_id = "app_info",
    responses(
        (status = 200, description = "Application info", body = AppInfoEnvelope),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    )
)]
#[get("")]
pub async fn info(state: web::Data<AppState>) -> Result<impl Responder, ApiError> {
    Ok(HttpResponse::Ok().json(AppInfoEnvelope {
        data: AppInfoResponse::new(state.environment.clone()),
    }))
}
