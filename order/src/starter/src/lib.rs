pub mod config;
pub mod wiring;

use crate::config::Settings;
use crate::wiring::{UseCases, outbox_relay, start_kafka_listener};
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

    // Blocking (connects and runs the migrations), which is fine before serving.
    let pool = shared::db::connect(&settings.db_settings(), infrastructure::MIGRATIONS)?;
    let use_cases = UseCases::new(&pool);
    let app_state = use_cases.app_state(&settings.environment);
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
        start_kafka_listener(
            &settings.kafka_host,
            &settings.kafka_group_id,
            use_cases.message_handler(),
            &shutdown,
        ),
    );
    workers.add(
        "Outbox relay",
        outbox_relay(&pool, &settings.kafka_host, settings.relay_settings()).spawn(&shutdown),
    );

    Ok(shared::web::serve(server, workers))
}
