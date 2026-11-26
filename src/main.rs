use anyhow::Result;
use tracing_subscriber::EnvFilter;
use urbaniac::{bot, config::Config, infrastructure::database};

#[tokio::main(worker_threads = 2)]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cfg = Config::from_env()?;
    let pool = database::connect(&cfg).await?;
    database::migrate(&pool).await?;
    tracing::info!("database siap, menjalankan bot");
    bot::run(cfg, pool).await
}
