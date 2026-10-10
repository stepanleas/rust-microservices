use anyhow::Result;
use api_gateway::config::Settings;
use std::io::IsTerminal;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    init_logging();

    let settings = Settings::from_env()?;
    let listener = TcpListener::bind(&settings.http_url).await?;
    tracing::info!("api gateway listening on {}", listener.local_addr()?);

    api_gateway::serve(listener, &settings).await
}

/// `RUST_LOG` filters (default `info`); `LOG_FORMAT=json` writes JSON lines for Loki, like the
/// services (`shared::logging`).
fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let json = std::env::var("LOG_FORMAT").is_ok_and(|format| format.eq_ignore_ascii_case("json"));

    if json {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .flatten_event(true)
            .with_current_span(false)
            .with_span_list(true)
            .init();
    } else {
        // No colour codes when the output goes to a file or `docker logs`.
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(std::io::stdout().is_terminal())
            .init();
    }
}
