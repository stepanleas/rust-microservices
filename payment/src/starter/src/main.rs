use anyhow::Result;
use starter::run;

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let _telemetry = shared::observability::init("payment");
    run().await?.await?;

    Ok(())
}
