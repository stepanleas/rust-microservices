use crate::dto::responses::{HealthCheckEnvelope, HealthCheckResponse};
use actix_web::{Error, HttpRequest, HttpResponse, Responder, get};

const HEALTH: &str = "Health";

fn healthy() -> HttpResponse {
    HttpResponse::Ok().json(HealthCheckEnvelope {
        data: HealthCheckResponse::new(),
    })
}

/// Startup probe.
///
/// Returns `Ok` once the application has started.
#[utoipa::path(
    tag = HEALTH,
    operation_id = "health_startup",
    responses(
        (status = 200, description = "Application has started", body = HealthCheckEnvelope)
    ),
)]
#[get("/startup")]
pub async fn startup(_: HttpRequest) -> actix_web::Result<impl Responder, Error> {
    Ok(healthy())
}

/// Liveness probe.
///
/// Returns `Ok` while the application is alive.
#[utoipa::path(
    tag = HEALTH,
    operation_id = "health_live",
    responses(
        (status = 200, description = "Application is alive", body = HealthCheckEnvelope)
    ),
)]
#[get("/live")]
pub async fn live(_: HttpRequest) -> actix_web::Result<impl Responder, Error> {
    Ok(healthy())
}

/// Readiness probe.
///
/// Returns `Ok` when the application is ready to handle requests.
#[utoipa::path(
    tag = HEALTH,
    operation_id = "health_ready",
    responses(
        (status = 200, description = "Application is ready", body = HealthCheckEnvelope)
    ),
)]
#[get("/ready")]
pub async fn ready(_: HttpRequest) -> actix_web::Result<impl Responder, Error> {
    Ok(healthy())
}
