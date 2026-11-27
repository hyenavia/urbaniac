use crate::application::{city, crew, inventory};
use crate::domain::activities::{
    def, roll_rewards_with_bonus, ActivityKind, BASE_ACTIVITY_SLOTS,
};
use crate::domain::city::CityStage;
use crate::domain::crew::apply_efficiency_duration;
use crate::domain::i18n::{activity_name, Lang};
use crate::domain::player::xp_to_next;
use crate::domain::progression::{activity_level_cap, apply_xp, eligible_rewards, XpState, NO_CAP};
use crate::domain::resources::{find_item, ItemDef};
use crate::infrastructure::repositories::{
    activity_repo, inventory_repo, player_repo, wallet_repo,
};
use crate::infrastructure::scheduler;
use chrono::{DateTime, Utc};
use rand::{rngs::StdRng, SeedableRng};
use rust_decimal::Decimal;
use sqlx::{PgConnection, PgPool};

pub const JOB_ACTIVITY_COMPLETE: &str = "activity_complete";

#[derive(Debug, thiserror::Error)]
pub enum ActivityError {
    #[error("profil belum ada, jalankan /start")]
    NotRegistered,
    #[error("aktivitas ini terbuka di stage {}", .0.name())]
    Locked(CityStage),
    #[error("semua Activity Slot terpakai ({0} slot). Ambil reward yang sudah selesai atau tunggu aktivitas berjalan.")]
    NoFreeSlot(i64),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Running,
    Ready,
}

#[derive(Debug, Clone)]
pub struct RunView {
    pub id: i64,
    pub kind: ActivityKind,
    pub status: RunStatus,
    pub ends_at: DateTime<Utc>,
}

pub struct ProgressView {
    pub kind: ActivityKind,
    pub state: XpState,
    pub cap: i32,
    pub xp_needed: i64,
}

pub struct ActivityOverview {
    pub slots_total: i64,
    pub runs: Vec<RunView>,
    pub progress: Vec<ProgressView>,
}

pub struct ClaimSummary {
    pub kind: ActivityKind,
    /// (item, masuk inventory, masuk Pending Claim)
    pub items: Vec<(&'static ItemDef, i64, i64)>,
    pub xp_gain: i64,
    pub old_level: i32,
    pub new_level: i32,
}

pub fn slot_total(_player_id: i64) -> i64 {
    BASE_ACTIVITY_SLOTS
}

/// Memulai aktivitas. `duration_pct` mengskala durasi (100 = normal; untuk testing).
pub async fn start(
    pool: &PgPool,
    player_id: i64,
    kind: ActivityKind,
    duration_pct: i64,
) -> Result<RunView, ActivityError> {
    let mut tx = pool.begin().await?;
    if !player_repo::lock(&mut *tx, player_id).await? {
        return Err(ActivityError::NotRegistered);
    }
    let stage = city::stage_for_conn(&mut *tx, player_id).await?;
    if stage < kind.min_stage() {
        return Err(ActivityError::Locked(kind.min_stage()));
    }
    let total = slot_total(player_id);
    if activity_repo::count_occupied(&mut *tx, player_id).await? >= total {
        return Err(ActivityError::NoFreeSlot(total));
    }
    let (crew_bonus, _) = crew::bonus_for_activity_conn(&mut *tx, player_id, kind).await?;
    let eff_secs = apply_efficiency_duration(def(kind).duration_secs, crew_bonus.efficiency_pct);
    let secs = (eff_secs * duration_pct.max(1) / 100).max(1);
    let (id, ends_at) = activity_repo::insert_run(&mut *tx, player_id, kind.as_str(), secs).await?;
    scheduler::enqueue(&mut *tx, JOB_ACTIVITY_COMPLETE, id, ends_at).await?;
    tx.commit().await?;
    Ok(RunView { id, kind, status: RunStatus::Running, ends_at })
}

pub async fn overview(pool: &PgPool, player_id: i64) -> anyhow::Result<ActivityOverview> {
    let mut conn = pool.acquire().await?;
    let stage = city::stage_for_conn(&mut *conn, player_id).await?;
    let rows = activity_repo::list_active(&mut *conn, player_id).await?;
    let runs = rows
        .into_iter()
        .filter_map(|r| {
            let kind = ActivityKind::parse(&r.activity)?;
            let status = if r.status == "ready" { RunStatus::Ready } else { RunStatus::Running };
            Some(RunView { id: r.id, kind, status, ends_at: r.ends_at })
        })
        .collect();
    let mut progress = Vec::new();
    for kind in ActivityKind::ALL {
        if stage < kind.min_stage() {
            continue;
        }
        let state = activity_repo::get_progress(&mut *conn, player_id, kind.as_str()).await?;
        progress.push(ProgressView {
            kind,
            state,
            cap: activity_level_cap(stage),
            xp_needed: xp_to_next(state.level),
        });
    }
    Ok(ActivityOverview { slots_total: slot_total(player_id), runs, progress })
}

/// Dipanggil worker di dalam transaksi job. Idempoten: hanya memproses run berstatus 'running'.
/// Jika kru terpasang punya `auto_enabled = true`, reward otomatis diklaim dan aktivitas
/// dimulai ulang pada slot yang sama (tanpa menambah Activity Slot).
pub async fn complete_run(
    conn: &mut PgConnection,
    run_id: i64,
    duration_pct: i64,
) -> anyhow::Result<Option<(i64, String)>> {
    let Some(run) = activity_repo::lock_run(conn, run_id).await? else {
        return Ok(None);
    };
    if run.status != "running" {
        return Ok(None);
    }
    let kind = ActivityKind::parse(&run.activity)
        .ok_or_else(|| anyhow::anyhow!("aktivitas tidak dikenal: {}", run.activity))?;
    let progress = activity_repo::get_progress(conn, run.player_id, kind.as_str()).await?;
    let (crew_bonus, auto_enabled) =
        crew::bonus_for_activity_conn(conn, run.player_id, kind).await?;
    let city_eff = city::effectiveness_for_conn(conn, run.player_id).await?;
    let city_yield_delta = ((city_eff - 100) / 2) as i64;
    let rewards = {
        let mut rng = StdRng::from_entropy();
        roll_rewards_with_bonus(
            def(kind),
            progress.level,
            crew_bonus.yield_pct + city_yield_delta,
            crew_bonus.luck_pct,
            &mut rng,
        )
    };
    for (item_id, qty) in rewards {
        activity_repo::insert_reward(conn, run_id, item_id, qty).await?;
    }
    activity_repo::mark_ready(conn, run_id).await?;

    let lang = player_repo::get_lang(conn, run.player_id)
        .await
        .unwrap_or(Lang::En);
    let act_label = activity_name(kind, lang);
    let mut note = match lang {
        Lang::En => format!(
            "<@{}> 🌾 Activity **{}** finished! Press **Collect Rewards** in the Activities menu.",
            run.player_id, act_label
        ),
        Lang::Id => format!(
            "<@{}> 🌾 Aktivitas **{}** selesai! Tekan **Ambil Reward** di menu Aktivitas.",
            run.player_id, act_label
        ),
    };
    if auto_enabled {
        let sum = claim_one_conn(conn, run.player_id, run_id).await?;
        let has_pending = sum
            .as_ref()
            .map(|s| s.items.iter().any(|(_, _, p)| *p > 0))
            .unwrap_or(false);
        let total = slot_total(run.player_id);
        if activity_repo::count_occupied(conn, run.player_id).await? < total {
            let eff_secs =
                apply_efficiency_duration(def(kind).duration_secs, crew_bonus.efficiency_pct);
            let secs = (eff_secs * duration_pct.max(1) / 100).max(1);
            let (new_id, ends_at) =
                activity_repo::insert_run(conn, run.player_id, kind.as_str(), secs).await?;
            scheduler::enqueue(conn, JOB_ACTIVITY_COMPLETE, new_id, ends_at).await?;
        }
        if has_pending {
            note = match lang {
                Lang::En => format!(
                    "<@{}> ⚠️ Activity **{}** finished (AUTO), but your **inventory is full** — some items went to **Pending Claim**!",
                    run.player_id, act_label
                ),
                Lang::Id => format!(
                    "<@{}> ⚠️ Aktivitas **{}** selesai (AUTO), tetapi **inventory penuh** — sebagian item masuk **Pending Claim**!",
                    run.player_id, act_label
                ),
            };
        } else {
            return Ok(None);
        }
    }
    Ok(Some((run.player_id, note)))
}

async fn claim_one_conn(
    conn: &mut PgConnection,
    player_id: i64,
    run_id: i64,
) -> anyhow::Result<Option<ClaimSummary>> {
    if !player_repo::lock(conn, player_id).await? {
        return Ok(None);
    }
    let Some(run) = activity_repo::lock_run(conn, run_id).await? else {
        return Ok(None);
    };
    if run.player_id != player_id || run.status != "ready" {
        return Ok(None);
    }
    let kind = ActivityKind::parse(&run.activity)
        .ok_or_else(|| anyhow::anyhow!("aktivitas tidak dikenal: {}", run.activity))?;

    let cap_slots = inventory::slot_capacity_conn(conn, player_id).await?;
    let mut items = Vec::new();
    for (item_id, qty) in activity_repo::list_rewards(conn, run_id).await? {
        let Some(item) = find_item(&item_id) else { continue };
        let p = inventory_repo::add_item(conn, player_id, item, qty, cap_slots).await?;
        items.push((item, p.stored, p.pending));
    }

    let xp_gain = def(kind).xp_reward;
    let cap = activity_level_cap(city::stage_for_conn(conn, player_id).await?);
    let old = activity_repo::lock_progress(conn, player_id, kind.as_str()).await?;
    let new = apply_xp(old, xp_gain, cap);
    activity_repo::save_progress(conn, player_id, kind.as_str(), new).await?;

    if let Some(p) = player_repo::get(conn, player_id).await? {
        let ps = apply_xp(XpState { level: p.level, xp: p.xp, overflow: 0 }, xp_gain, NO_CAP);
        player_repo::set_progress(conn, player_id, ps.level, ps.xp).await?;
        for r in eligible_rewards(ps.level) {
            if player_repo::claim_progression_reward_if_absent(conn, player_id, r.key).await? {
                let amount = Decimal::new(r.bac_cents, 2);
                let _ = wallet_repo::apply(
                    conn,
                    player_id,
                    amount,
                    &format!("progression_reward:{}", r.key),
                    &format!("prog_reward:{player_id}:{}", r.key),
                )
                .await;
            }
        }
    }

    activity_repo::mark_claimed(conn, run_id).await?;
    Ok(Some(ClaimSummary { kind, items, xp_gain, old_level: old.level, new_level: new.level }))
}

/// Menerapkan XP overflow semua aktivitas pemain saat stage kota naik.
pub async fn flush_overflow_for_stage_conn(
    conn: &mut PgConnection,
    player_id: i64,
    new_stage: CityStage,
) -> sqlx::Result<Vec<(ActivityKind, i32, i32)>> {
    let cap = activity_level_cap(new_stage);
    let mut leveled = Vec::new();
    for act_name in activity_repo::list_player_activities(conn, player_id).await? {
        let Some(kind) = ActivityKind::parse(&act_name) else { continue };
        let old = activity_repo::lock_progress(conn, player_id, kind.as_str()).await?;
        if old.overflow == 0 {
            continue;
        }
        let new = apply_xp(old, 0, cap);
        if new != old {
            activity_repo::save_progress(conn, player_id, kind.as_str(), new).await?;
            if new.level > old.level {
                leveled.push((kind, old.level, new.level));
            }
        }
    }
    Ok(leveled)
}

async fn claim_one(pool: &PgPool, player_id: i64, run_id: i64) -> anyhow::Result<Option<ClaimSummary>> {
    let mut tx = pool.begin().await?;
    let res = claim_one_conn(&mut *tx, player_id, run_id).await?;
    tx.commit().await?;
    Ok(res)
}

/// Mengambil semua reward yang sudah siap. Tiap run diproses dalam transaksinya sendiri.
pub async fn collect_ready(pool: &PgPool, player_id: i64) -> anyhow::Result<Vec<ClaimSummary>> {
    let ids = {
        let mut conn = pool.acquire().await?;
        activity_repo::list_ready_ids(&mut *conn, player_id).await?
    };
    let mut out = Vec::new();
    for id in ids {
        if let Some(s) = claim_one(pool, player_id, id).await? {
            out.push(s);
        }
    }
    Ok(out)
}
