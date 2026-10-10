pub mod config;
pub mod wiring;

use crate::config::Settings;
use crate::wiring::{UseCases, outbox_relay, start_kafka_listener};
use actix_web::dev::Server;
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::{App, HttpServer};
use anyhow::Result;
use shared::web::CorrelationIdMiddleware;
use tracing_actix_web::TracingLogger;
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;

pub async fn run() -> Result<Server> {
    run_internal(&Settings::new()).await
}

async fn run_internal(settings: &Settings) -> Result<Server> {
    tracing::info!("Starting HTTP server at {}", settings.http_url);
    tracing::debug!("with configuration: {:?}", settings);

    // Blocking (connects and runs the migrations), which is fine before serving.
    let pool = shared::db::connect(&settings.db_settings(), infrastructure::MIGRATIONS)?;
    let use_cases = UseCases::new(&pool);
    let app_state = use_cases.app_state(&settings.environment);

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
            .into_app()
    })
    .bind(&settings.http_url)?
    .run();

    start_kafka_listener(
        &settings.kafka_host,
        &settings.kafka_group_id,
        use_cases.message_handler(),
    );
    outbox_relay(&pool, &settings.kafka_host, settings.relay_settings()).spawn();

    Ok(server)
}
