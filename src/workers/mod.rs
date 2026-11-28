//! Scheduler tunggal + worker ber-batas. Tidak ada timer per aktivitas dan tidak ada polling rapat:
//! scheduler tidur sampai job berikutnya jatuh tempo (atau dibangunkan lewat Notify).
use crate::application::voice_boost::VoiceBoostTracker;
use crate::application::{activities, city, crafting, giveaway, market};
use crate::infrastructure::scheduler::{self, DueJob};
use crate::web::events::{EventHub, RealtimeEvent};
use chrono::Utc;
use serenity::all::{CreateMessage, Http, UserId};
use sqlx::PgPool;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinSet;

pub const BATCH_SIZE: i64 = 25;
pub const WORKER_CONCURRENCY: usize = 2;
const MAX_IDLE: Duration = Duration::from_secs(60);
const ERROR_BACKOFF: Duration = Duration::from_secs(5);

async fn send_notifications(http_slot: &OnceLock<Arc<Http>>, notes: Vec<(i64, String)>) {
    let Some(http) = http_slot.get() else {
        return;
    };
    let components = crate::bot::components::dm_activity_button();
    for (player_id, msg) in notes {
        let user = UserId::new(player_id as u64);
        match user.create_dm_channel(http).await {
            Ok(dm) => {
                let dm_msg = CreateMessage::new()
                    .content(msg)
                    .components(components.clone());
                if let Err(e) = dm.send_message(http, dm_msg).await {
                    tracing::debug!(player_id, error = ?e, "gagal kirim DM notifikasi");
                }
            }
            Err(e) => {
                tracing::debug!(player_id, error = ?e, "tidak bisa membuka DM pemain");
            }
        }
    }
}

async fn run_job(
    pool: &PgPool,
    job: &DueJob,
    duration_pct: i64,
    http_slot: &OnceLock<Arc<Http>>,
    events: Option<&EventHub>,
    voice_boost: Option<&VoiceBoostTracker>,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    if !scheduler::lock_pending(&mut *tx, job.id).await? {
        return Ok(());
    }
    let mut giveaway_result = None;
    let notes = match job.kind.as_str() {
        activities::JOB_ACTIVITY_COMPLETE => {
            activities::complete_run_with_voice_boost(&mut *tx, job.ref_id, duration_pct, voice_boost)
                .await?
                .into_iter()
                .collect()
        }
        city::JOB_CITY_PROJECT_COMPLETE => {
            city::complete_project(&mut *tx, job.ref_id)
                .await?
                .into_iter()
                .collect()
        }
        city::JOB_CITY_ECONOMY_TICK => {
            city::handle_city_economy_tick_job(&mut *tx, job.ref_id, duration_pct).await?
        }
        crafting::JOB_CRAFT_COMPLETE => {
            crafting::complete_craft_with_voice_boost(&mut *tx, job.ref_id, voice_boost)
                .await?
                .into_iter()
                .collect()
        }
        market::JOB_BROKER_EXPIRE => {
            market::expire_listing(&mut *tx, job.ref_id)
                .await?
                .into_iter()
                .collect()
        }
        market::JOB_MARKET_TICK => {
            market::handle_market_tick_job(&mut *tx, job.ref_id, duration_pct).await?
        }
        giveaway::JOB_GIVEAWAY_END => {
            giveaway_result = giveaway::finalize_giveaway_conn(&mut *tx, job.ref_id).await?;
            Vec::new()
        }
        other => anyhow::bail!("jenis job tidak dikenal: {other}"),
    };
    scheduler::mark_done(&mut *tx, job.id).await?;
    tx.commit().await?;

    if let Some(res) = &giveaway_result {
        if let Some(hub) = events {
            for &pid in &res.credited_winners {
                hub.publish(RealtimeEvent {
                    player_id: Some(pid),
                    kind: "giveaway_won",
                    message: Some(format!(
                        "You won {} (Giveaway #{})!",
                        res.giveaway.prize, res.giveaway.id
                    )),
                });
            }
        }
        if let Some(http) = http_slot.get() {
            crate::bot::giveaway_ui::publish_giveaway_result(http, res).await;
        }
    }

    if let Some(hub) = events {
        let ev_kind = match job.kind.as_str() {
            activities::JOB_ACTIVITY_COMPLETE => "activity_complete",
            city::JOB_CITY_PROJECT_COMPLETE => "city_project_complete",
            city::JOB_CITY_ECONOMY_TICK => "city_economy_tick",
            crafting::JOB_CRAFT_COMPLETE => "craft_complete",
            market::JOB_BROKER_EXPIRE => "broker_expire",
            market::JOB_MARKET_TICK => "market_tick",
            _ => "state_sync",
        };
        if notes.is_empty() {
            if matches!(
                job.kind.as_str(),
                market::JOB_MARKET_TICK
                    | city::JOB_CITY_ECONOMY_TICK
                    | activities::JOB_ACTIVITY_COMPLETE
            ) {
                hub.publish(RealtimeEvent {
                    player_id: None,
                    kind: "state_sync",
                    message: None,
                });
            }
        } else {
            for (pid, msg) in &notes {
                hub.publish(RealtimeEvent {
                    player_id: Some(*pid),
                    kind: ev_kind,
                    message: Some(msg.replace("**", "")),
                });
            }
            if matches!(
                job.kind.as_str(),
                market::JOB_MARKET_TICK | city::JOB_CITY_ECONOMY_TICK
            ) {
                hub.publish(RealtimeEvent {
                    player_id: None,
                    kind: "state_sync",
                    message: None,
                });
            }
        }
    }

    send_notifications(http_slot, notes).await;
    Ok(())
}

async fn process_job(
    pool: &PgPool,
    job: &DueJob,
    duration_pct: i64,
    http_slot: &OnceLock<Arc<Http>>,
    events: Option<&EventHub>,
    voice_boost: Option<&VoiceBoostTracker>,
) {
    if let Err(e) = run_job(pool, job, duration_pct, http_slot, events, voice_boost).await {
        tracing::error!(job_id = job.id, error = ?e, "job gagal");
        if let Ok(mut conn) = pool.acquire().await {
            let _ = scheduler::record_failure(&mut *conn, job.id, &format!("{e:#}")).await;
        }
    }
}

/// Memproses satu batch job yang jatuh tempo (maks BATCH_SIZE, WORKER_CONCURRENCY paralel).
/// Mengembalikan jumlah job yang diambil.
pub async fn process_due_batch(pool: &PgPool) -> anyhow::Result<usize> {
    let empty = Arc::new(OnceLock::new());
    process_due_batch_with_pct(pool, 100, &empty).await
}

pub async fn process_due_batch_with_pct(
    pool: &PgPool,
    duration_pct: i64,
    http_slot: &Arc<OnceLock<Arc<Http>>>,
) -> anyhow::Result<usize> {
    process_due_batch_internal(pool, duration_pct, http_slot, None, None).await
}

async fn process_due_batch_internal(
    pool: &PgPool,
    duration_pct: i64,
    http_slot: &Arc<OnceLock<Arc<Http>>>,
    events: Option<&Arc<EventHub>>,
    voice_boost: Option<&Arc<VoiceBoostTracker>>,
) -> anyhow::Result<usize> {
    let jobs = {
        let mut conn = pool.acquire().await?;
        scheduler::due_jobs(&mut *conn, BATCH_SIZE).await?
    };
    let n = jobs.len();
    for chunk in jobs.chunks(WORKER_CONCURRENCY) {
        let mut set = JoinSet::new();
        for job in chunk {
            let pool = pool.clone();
            let job = job.clone();
            let http = http_slot.clone();
            let ev = events.cloned();
            let vb = voice_boost.cloned();
            set.spawn(async move {
                process_job(&pool, &job, duration_pct, &http, ev.as_deref(), vb.as_deref()).await
            });
        }
        while let Some(res) = set.join_next().await {
            if let Err(e) = res {
                tracing::error!(error = ?e, "worker panic");
            }
        }
    }
    Ok(n)
}

pub async fn run_scheduler(
    pool: PgPool,
    notify: Arc<Notify>,
    duration_pct: i64,
    http_slot: Arc<OnceLock<Arc<Http>>>,
) {
    run_scheduler_with_events(pool, notify, duration_pct, http_slot, None, None).await;
}

pub async fn run_scheduler_with_events(
    pool: PgPool,
    notify: Arc<Notify>,
    duration_pct: i64,
    http_slot: Arc<OnceLock<Arc<Http>>>,
    events: Option<Arc<EventHub>>,
    voice_boost: Option<Arc<VoiceBoostTracker>>,
) {
    tracing::info!("scheduler berjalan");
    if let Err(e) = market::ensure_market_tick_scheduled(&pool, duration_pct).await {
        tracing::error!(error = ?e, "gagal menjadwalkan market_tick awal");
    }
    if let Err(e) = city::ensure_city_economy_tick_scheduled(&pool, duration_pct).await {
        tracing::error!(error = ?e, "gagal menjadwalkan city_economy_tick awal");
    }
    loop {
        match process_due_batch_internal(
            &pool,
            duration_pct,
            &http_slot,
            events.as_ref(),
            voice_boost.as_ref(),
        )
        .await
        {
            Ok(n) if n > 0 => continue, // mungkin masih ada yang jatuh tempo
            Ok(_) => {}
            Err(e) => {
                tracing::error!(error = ?e, "scheduler error");
                tokio::time::sleep(ERROR_BACKOFF).await;
                continue;
            }
        }
        let wait = match pool.acquire().await {
            Ok(mut conn) => match scheduler::next_run_at(&mut *conn).await {
                Ok(Some(t)) => (t - Utc::now()).to_std().unwrap_or(Duration::ZERO).min(MAX_IDLE),
                Ok(None) => MAX_IDLE,
                Err(_) => ERROR_BACKOFF,
            },
            Err(_) => ERROR_BACKOFF,
        };
        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            _ = notify.notified() => {}
        }
    }
}
