pub mod config;
pub mod wiring;

use crate::config::Settings;
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::{App, HttpServer};
use anyhow::Result;
use shared::db::DbHealthCheck;
use shared::shutdown::{Shutdown, Workers};
use shared::web::CorrelationIdMiddleware;
use shared::web::health::HealthChecks;
use tracing_actix_web::TracingLogger;
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;

pub async fn run() -> Result<impl Future<Output = std::io::Result<()>> + use<>> {
    run_internal(&Settings::new()).await
}

async fn run_internal(
    settings: &Settings,
) -> Result<impl Future<Output = std::io::Result<()>> + use<>> {
    tracing::info!("Starting HTTP server at {}", settings.http_url);
    tracing::debug!("with configuration: {:?}", settings);

    let pool = wiring::connect_database(settings)?;
    let app_state = wiring::build_app_state(settings, &pool);
    let health_checks = HealthChecks::new().with(DbHealthCheck::new(&pool));

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
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
    })
    .bind(&settings.http_url)?
    .run();

    let shutdown = Shutdown::new();
    let mut workers = Workers::new(shutdown.clone());
    workers.add(
        "Kafka listener",
        wiring::spawn_kafka_listener(settings, &pool, &shutdown),
    );
    workers.add(
        "Outbox relay",
        wiring::spawn_outbox_relay(settings, &pool, &shutdown),
    );

    Ok(shared::web::serve(server, workers))
}
