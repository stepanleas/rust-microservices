//! Kubernetes probes, mounted by every service under `/api/health`.
//!
//! - `startup`: the service runs its migrations before it binds the HTTP port, so answering at
//!   all means startup is done.
//! - `live`: the process is up. Never checks dependencies, or a database outage would make
//!   Kubernetes restart every pod.
//! - `ready`: runs the [`HealthChecks`] registered as app data and answers 503 when one fails,
//!   so the pod is taken out of the Service until the dependency is back. Without registered
//!   checks the service is always ready.

use crate::health::HealthCheck;
use actix_web::{HttpResponse, get, web};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use utoipa::ToSchema;

const HEALTH: &str = "Health";
const OK: &str = "Ok";
const UNAVAILABLE: &str = "Unavailable";

/// The dependencies checked by the readiness probe. Register it with `App::app_data(Data::new(..))`.
#[derive(Clone, Default)]
pub struct HealthChecks {
    checks: Vec<Arc<dyn HealthCheck>>,
}

impl HealthChecks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, check: impl HealthCheck + 'static) -> Self {
        self.checks.push(Arc::new(check));
        self
    }

    /// Runs every check (blocking) and returns the status of each one by name.
    fn run(&self) -> (bool, BTreeMap<String, String>) {
        let mut all_ok = true;
        let mut statuses = BTreeMap::new();

        for check in &self.checks {
            let status = match check.check() {
                Ok(()) => OK,
                Err(reason) => {
                    tracing::warn!("Readiness check {} failed: {reason}", check.name());
                    all_ok = false;
                    UNAVAILABLE
                }
            };
            statuses.insert(check.name().to_string(), status.to_string());
        }

        (all_ok, statuses)
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthStatus {
    #[schema(example = "Ok")]
    pub status: String,
    /// Status of each dependency, only on the readiness probe.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[schema(example = json!({ "database": "Ok" }))]
    pub checks: BTreeMap<String, String>,
}

/// `{ "data": HealthStatus }` envelope.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub data: HealthStatus,
}

fn respond(healthy: bool, checks: BTreeMap<String, String>) -> HttpResponse {
    let status = if healthy { OK } else { UNAVAILABLE };
    let body = HealthResponse {
        data: HealthStatus {
            status: status.to_string(),
            checks,
        },
    };

    if healthy {
        HttpResponse::Ok().json(body)
    } else {
        HttpResponse::ServiceUnavailable().json(body)
    }
}

/// Startup probe.
///
/// Returns `Ok` once the application has started (migrations applied).
#[utoipa::path(
    tag = HEALTH,
    operation_id = "health_startup",
    responses(
        (status = 200, description = "Application has started", body = HealthResponse)
    ),
)]
#[get("/startup")]
pub async fn startup() -> HttpResponse {
    respond(true, BTreeMap::new())
}

/// Liveness probe.
///
/// Returns `Ok` while the process is alive. Does not check dependencies.
#[utoipa::path(
    tag = HEALTH,
    operation_id = "health_live",
    responses(
        (status = 200, description = "Application is alive", body = HealthResponse)
    ),
)]
#[get("/live")]
pub async fn live() -> HttpResponse {
    respond(true, BTreeMap::new())
}

/// Readiness probe.
///
/// Returns `Ok` when every dependency (e.g. the database) is usable, 503 otherwise.
#[utoipa::path(
    tag = HEALTH,
    operation_id = "health_ready",
    responses(
        (status = 200, description = "Application is ready", body = HealthResponse),
        (status = 503, description = "A dependency is unavailable", body = HealthResponse)
    ),
)]
#[get("/ready")]
pub async fn ready(checks: Option<web::Data<HealthChecks>>) -> HttpResponse {
    let Some(checks) = checks else {
        return respond(true, BTreeMap::new());
    };

    let checks = checks.get_ref().clone();
    match web::block(move || checks.run()).await {
        Ok((healthy, statuses)) => respond(healthy, statuses),
        Err(error) => {
            tracing::error!("Readiness checks could not run: {error}");
            respond(false, BTreeMap::new())
        }
    }
}

/// Registers the three probes; mount it under `/api/health`.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(startup).service(live).service(ready);
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test, web::Data};
    use serde_json::Value;

    struct Fixed(Result<(), String>);

    impl HealthCheck for Fixed {
        fn name(&self) -> &'static str {
            "database"
        }

        fn check(&self) -> Result<(), String> {
            self.0.clone()
        }
    }

    async fn get(checks: Option<HealthChecks>, uri: &str) -> (u16, Value) {
        let mut app = App::new();
        if let Some(checks) = checks {
            app = app.app_data(Data::new(checks));
        }
        let app =
            test::init_service(app.service(web::scope("/api/health").configure(configure))).await;
        let response =
            test::call_service(&app, test::TestRequest::get().uri(uri).to_request()).await;
        let status = response.status().as_u16();
        (status, test::read_body_json(response).await)
    }

    #[actix_web::test]
    async fn test_ready_without_checks_is_ok() {
        let (status, body) = get(None, "/api/health/ready").await;

        assert_eq!(200, status);
        assert_eq!("Ok", body["data"]["status"]);
    }

    #[actix_web::test]
    async fn test_ready_reports_each_passing_check() {
        let checks = HealthChecks::new().with(Fixed(Ok(())));

        let (status, body) = get(Some(checks), "/api/health/ready").await;

        assert_eq!(200, status);
        assert_eq!("Ok", body["data"]["status"]);
        assert_eq!("Ok", body["data"]["checks"]["database"]);
    }

    #[actix_web::test]
    async fn test_ready_is_unavailable_when_a_check_fails() {
        let checks = HealthChecks::new().with(Fixed(Err("connection refused".into())));

        let (status, body) = get(Some(checks), "/api/health/ready").await;

        assert_eq!(503, status);
        assert_eq!("Unavailable", body["data"]["status"]);
        assert_eq!("Unavailable", body["data"]["checks"]["database"]);
    }

    #[actix_web::test]
    async fn test_live_and_startup_ignore_failing_checks() {
        for uri in ["/api/health/live", "/api/health/startup"] {
            let checks = HealthChecks::new().with(Fixed(Err("down".into())));

            let (status, body) = get(Some(checks), uri).await;

            assert_eq!(200, status, "{uri}");
            assert_eq!("Ok", body["data"]["status"], "{uri}");
            assert!(body["data"].get("checks").is_none(), "{uri}");
        }
    }
}
