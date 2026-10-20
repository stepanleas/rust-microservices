pub mod config;
pub mod wiring;

use crate::config::Settings;
use actix_web::dev::Server;
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::{App, HttpServer, web};
use anyhow::Result;
use shared::db::DbHealthCheck;
use shared::web::health::HealthChecks;
use shared::web::{CorrelationIdMiddleware, HttpMetrics, METRICS_PATH, metrics_endpoint};
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;

pub async fn run() -> Result<Server> {
    run_internal(&Settings::new()).await
}

async fn run_internal(settings: &Settings) -> Result<Server> {
    tracing::info!("Starting HTTP server at {}", settings.http_url);
    tracing::debug!("with configuration: {:?}", settings);

    let pool = wiring::connect_database(settings)?;
    let app_state = wiring::build_app_state(settings, &pool);
    let health_checks = HealthChecks::new().with(DbHealthCheck::new(&pool));

    let server = HttpServer::new(move || {
        App::new()
            .wrap(HttpMetrics)
            .wrap(CorrelationIdMiddleware)
            .into_utoipa_app()
            .openapi(presentation::api::docs::open_api_docs())
            .map(|app| app.wrap(Logger::default()))
            .map(|app| app.configure(presentation::api::configure))
            .openapi_service(|api| {
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", api)
            })
            .app_data(Data::new(app_state.clone()))
            .app_data(Data::new(health_checks.clone()))
            .into_app()
            .route(METRICS_PATH, web::get().to(metrics_endpoint))
    })
    .bind(&settings.http_url)?
    .run();

    Ok(server)
}
