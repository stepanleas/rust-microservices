use crate::dto::responses::{HealthCheckDataResponse, HealthCheckResponse};
use actix_web::{Error, HttpRequest, HttpResponse, Responder, get};

const HEALTH: &str = "Health";

/// Startup probe.
///
/// Returns `Ok` once the application has started.
#[utoipa::path(
    tag = HEALTH,
    operation_id = "health_startup",
    responses(
        (status = 200, description = "Application has started", body = HealthCheckDataResponse)
    ),
)]
#[get("/startup")]
pub async fn startup(_: HttpRequest) -> actix_web::Result<impl Responder, Error> {
    Ok(HttpResponse::Ok().json(HealthCheckDataResponse {
        data: HealthCheckResponse::new(),
    }))
}

/// Liveness probe.
///
/// Returns `Ok` while the application is alive.
#[utoipa::path(
    tag = HEALTH,
    operation_id = "health_live",
    responses(
        (status = 200, description = "Application is alive", body = HealthCheckDataResponse)
    ),
)]
#[get("/live")]
pub async fn live(_: HttpRequest) -> actix_web::Result<impl Responder, Error> {
    Ok(HttpResponse::Ok().json(HealthCheckDataResponse {
        data: HealthCheckResponse::new(),
    }))
}

/// Readiness probe.
///
/// Returns `Ok` when the application is ready to serve requests.
#[utoipa::path(
    tag = HEALTH,
    operation_id = "health_ready",
    responses(
        (status = 200, description = "Application is ready", body = HealthCheckDataResponse)
    ),
)]
#[get("/ready")]
pub async fn ready(_: HttpRequest) -> actix_web::Result<impl Responder, Error> {
    Ok(HttpResponse::Ok().json(HealthCheckDataResponse {
        data: HealthCheckResponse::new(),
    }))
}
