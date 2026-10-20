use crate::shutdown::Workers;
use actix_web::dev::Server;
use std::time::Duration;

/// How long the background threads get to finish their message or batch after the HTTP
/// server stopped. With actix's 30 s for in-flight requests, Kubernetes'
/// `terminationGracePeriodSeconds` should be at least 50.
pub const WORKERS_STOP_TIMEOUT: Duration = Duration::from_secs(15);

/// Serves until SIGTERM/SIGINT (actix then stops accepting connections and finishes the
/// in-flight requests), then stops the service's background threads.
pub async fn serve(server: Server, workers: Workers) -> std::io::Result<()> {
    let result = server.await;
    tracing::info!("HTTP server stopped, stopping background workers");
    workers.stop(WORKERS_STOP_TIMEOUT);
    result
}
