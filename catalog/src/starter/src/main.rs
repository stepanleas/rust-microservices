use anyhow::Result;
use starter::run;

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    shared::logging::init();
    run().await?.await?;

    Ok(())
}
