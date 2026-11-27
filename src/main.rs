use anyhow::Result;
use serenity::all::Http;
use std::sync::{Arc, OnceLock};
use tokio::sync::Notify;
use tracing_subscriber::EnvFilter;
use urbaniac::{bot, config::Config, infrastructure::database, workers};

#[tokio::main(worker_threads = 2)]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cfg = Config::from_env()?;
    let pool = database::connect(&cfg).await?;
    database::migrate(&pool).await?;

    // Satu instance scheduler. Job yang tertunda saat bot mati akan diproses saat start.
    let notify = Arc::new(Notify::new());
    let http_slot: Arc<OnceLock<Arc<Http>>> = Arc::new(OnceLock::new());
    tokio::spawn(workers::run_scheduler(
        pool.clone(),
        notify.clone(),
        cfg.activity_duration_pct,
        http_slot.clone(),
    ));

    tracing::info!("database siap, menjalankan bot");
    bot::run(cfg, pool, notify, http_slot).await
}
