use anyhow::Result;
use starter::run;

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let _telemetry = shared::observability::init("catalog");
    run().await?.await?;

    Ok(())
}
