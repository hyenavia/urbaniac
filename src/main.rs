use anyhow::Result;
use serenity::all::Http;
use std::sync::{Arc, OnceLock};
use tokio::sync::Notify;
use tracing_subscriber::EnvFilter;
use urbaniac::application::voice_boost::VoiceBoostTracker;
use urbaniac::web::events::EventHub;
use urbaniac::{bot, config::Config, infrastructure::database, web, workers};

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
    let voice_boost = Arc::new(VoiceBoostTracker::new(
        cfg.guild_id,
        cfg.voice_boost_channel_id,
    ));
    let events = if cfg.activity.enabled {
        Some(Arc::new(EventHub::default()))
    } else {
        None
    };

    tokio::spawn(workers::run_scheduler_with_events(
        pool.clone(),
        notify.clone(),
        cfg.activity_duration_pct,
        http_slot.clone(),
        events.clone(),
        Some(voice_boost.clone()),
    ));

    if let Some(hub) = events.clone() {
        let activity_cfg = cfg.activity.clone();
        let bind_addr = activity_cfg.bind_addr.clone();
        let web_pool = pool.clone();
        let web_notify = notify.clone();
        let duration_pct = cfg.activity_duration_pct;
        let web_voice_boost = voice_boost.clone();
        tokio::spawn(async move {
            if let Err(e) = web::run(
                activity_cfg,
                web_pool,
                web_notify,
                duration_pct,
                hub,
                web_voice_boost,
            )
            .await
            {
                tracing::error!(
                    bind_addr = %bind_addr,
                    error = ?e,
                    "web Activity API berhenti dengan error"
                );
            }
        });
    }

    tracing::info!("database siap, menjalankan bot");
    bot::run(cfg, pool, notify, http_slot, voice_boost, events).await
}
