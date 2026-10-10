use anyhow::Result;
use api_gateway::config::Settings;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let settings = Settings::from_env()?;
    let listener = TcpListener::bind(&settings.http_url).await?;
    tracing::info!("api gateway listening on {}", listener.local_addr()?);

    api_gateway::serve(listener, &settings).await
}
