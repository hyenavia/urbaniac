//! Scheduler tunggal + worker ber-batas. Tidak ada timer per aktivitas dan tidak ada polling rapat:
//! scheduler tidur sampai job berikutnya jatuh tempo (atau dibangunkan lewat Notify).
use crate::application::{activities, city, crafting, market};
use crate::infrastructure::repositories::player_repo;
use crate::infrastructure::scheduler::{self, DueJob};
use chrono::Utc;
use serenity::all::{ChannelId, Http};
use sqlx::PgPool;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinSet;

pub const BATCH_SIZE: i64 = 25;
pub const WORKER_CONCURRENCY: usize = 2;
const MAX_IDLE: Duration = Duration::from_secs(60);
const ERROR_BACKOFF: Duration = Duration::from_secs(5);

async fn send_notifications(
    pool: &PgPool,
    http_slot: &OnceLock<Arc<Http>>,
    notes: Vec<(i64, String)>,
) {
    let Some(http) = http_slot.get() else {
        return;
    };
    for (player_id, msg) in notes {
        let Ok(mut conn) = pool.acquire().await else { continue };
        if let Ok(Some(tid)) = player_repo::get_thread_id(&mut *conn, player_id).await {
            let _ = ChannelId::new(tid as u64).say(http, msg).await;
        }
    }
}

async fn run_job(
    pool: &PgPool,
    job: &DueJob,
    duration_pct: i64,
    http_slot: &OnceLock<Arc<Http>>,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    if !scheduler::lock_pending(&mut *tx, job.id).await? {
        return Ok(());
    }
    let notes = match job.kind.as_str() {
        activities::JOB_ACTIVITY_COMPLETE => {
            activities::complete_run(&mut *tx, job.ref_id, duration_pct)
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
        crafting::JOB_CRAFT_COMPLETE => {
            crafting::complete_craft(&mut *tx, job.ref_id)
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
            market::handle_market_tick_job(&mut *tx, duration_pct).await?
        }
        other => anyhow::bail!("jenis job tidak dikenal: {other}"),
    };
    scheduler::mark_done(&mut *tx, job.id).await?;
    tx.commit().await?;
    send_notifications(pool, http_slot, notes).await;
    Ok(())
}

async fn process_job(
    pool: &PgPool,
    job: &DueJob,
    duration_pct: i64,
    http_slot: &OnceLock<Arc<Http>>,
) {
    if let Err(e) = run_job(pool, job, duration_pct, http_slot).await {
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
            set.spawn(async move { process_job(&pool, &job, duration_pct, &http).await });
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
    tracing::info!("scheduler berjalan");
    if let Err(e) = market::ensure_market_tick_scheduled(&pool, duration_pct).await {
        tracing::error!(error = ?e, "gagal menjadwalkan market_tick awal");
    }
    loop {
        match process_due_batch_with_pct(&pool, duration_pct, &http_slot).await {
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
