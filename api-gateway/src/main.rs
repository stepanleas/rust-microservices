use anyhow::Result;
use api_gateway::config::Settings;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    // `RUST_LOG`, `LOG_FORMAT=json` and the `OTEL_*` variables work as in the services.
    let _telemetry = shared::observability::init("api-gateway");

    let settings = Settings::from_env()?;
    let listener = TcpListener::bind(&settings.http_url).await?;
    tracing::info!("api gateway listening on {}", listener.local_addr()?);

    api_gateway::serve(listener, &settings).await
}
