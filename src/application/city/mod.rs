use crate::domain::activities::ActivityKind;
use crate::domain::city::{
    def, local_modifier_from_city, project_cost_bac_for_level, settle_city_economy_tick,
    yield_bonus_from_effectiveness, CityStage, ProjectId, CITY_ECONOMY_TICK_SECS, MAX_TAX_RATE,
    MIN_TAX_RATE,
};
use crate::domain::i18n::{activity_name, project_name, stage_name, Lang};
use crate::domain::resources::find_item;
use crate::infrastructure::repositories::wallet_repo::{self, ApplyOutcome, WalletError};
use crate::infrastructure::repositories::{city_repo, crew_repo, inventory_repo, player_repo};
use crate::infrastructure::scheduler;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use sqlx::{PgConnection, PgPool};

pub const JOB_CITY_PROJECT_COMPLETE: &str = "city_project_complete";
pub const JOB_CITY_ECONOMY_TICK: &str = "city_economy_tick";

#[derive(Debug, thiserror::Error)]
pub enum CityError {
    #[error("No profile yet, run /start")]
    NotRegistered,
    #[error("Project {0} is already in progress")]
    ProjectAlreadyRunning(String),
    #[error("Facility {0} has already reached maximum level")]
    AlreadyBuilt(&'static str),
    #[error("This project requires city stage {}", .0.name())]
    StageTooLow(CityStage),
    #[error("Facility prerequisites have not been met")]
    MissingPrerequisites,
    #[error("This stage upgrade project has already been completed")]
    StageAlreadyAdvanced,
    #[error("Insufficient Bac balance (need {need}, have {have})")]
    InsufficientBac { need: Decimal, have: Decimal },
    #[error("Insufficient City Treasury balance (need {need}, have {have})")]
    InsufficientTreasury { need: Decimal, have: Decimal },
    #[error("Insufficient resource {name} (need {need}, have {have})")]
    InsufficientItem { name: &'static str, need: i64, have: i64 },
    #[error("Amount must be greater than 0")]
    InvalidAmount,
    #[error("Tax rate must be between {MIN_TAX_RATE}% and {MAX_TAX_RATE}%")]
    InvalidTaxRate,
    #[error(transparent)]
    Wallet(#[from] WalletError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

pub struct FacilityView {
    pub id: ProjectId,
    pub level: i32,
}

pub struct RunningProjectView {
    pub id: i64,
    pub project: ProjectId,
    pub ends_at: DateTime<Utc>,
}

pub struct CityOverview {
    pub stage: CityStage,
    pub treasury: Decimal,
    pub wallet_balance: Decimal,
    pub tax_rate_pct: i32,
    pub welfare: i32,
    pub effectiveness_pct: i32,
    pub local_modifier: Decimal,
    pub est_tax_income: Decimal,
    pub est_expense: Decimal,
    pub assigned_crews: i64,
    pub yield_bonus_pct: i64,
    pub facilities: Vec<FacilityView>,
    pub running_project: Option<RunningProjectView>,
}

pub struct ProjectQuoteItem {
    pub item_id: &'static str,
    pub need: i64,
    pub have: i64,
}

pub struct ProjectQuoteView {
    pub project: ProjectId,
    pub current_level: i32,
    pub next_level: Option<i32>,
    pub max_level: i32,
    pub min_stage: CityStage,
    pub cost_bac: Decimal,
    pub available_bac: Decimal,
    pub duration_secs: i64,
    pub items: Vec<ProjectQuoteItem>,
    pub can_start: bool,
    pub block_reason: Option<String>,
}

/// Mengambil CityStage pemain dari database (default Kampung jika belum ada).
pub async fn stage_for_conn(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<CityStage> {
    let raw = city_repo::get_stage(conn, player_id).await?;
    Ok(raw.as_deref().and_then(CityStage::parse).unwrap_or(CityStage::Kampung))
}

pub async fn stage_for(pool: &PgPool, player_id: i64) -> anyhow::Result<CityStage> {
    let mut conn = pool.acquire().await?;
    Ok(stage_for_conn(&mut *conn, player_id).await?)
}

pub async fn overview(pool: &PgPool, player_id: i64) -> anyhow::Result<CityOverview> {
    let mut conn = pool.acquire().await?;
    let city = city_repo::get(&mut *conn, player_id).await?;
    let (stage, treasury, tax_rate_pct, welfare, effectiveness_pct, local_modifier) = match city {
        Some(c) => (
            CityStage::parse(&c.stage).unwrap_or(CityStage::Kampung),
            c.treasury,
            c.tax_rate_pct,
            c.welfare,
            c.effectiveness_pct,
            c.local_modifier,
        ),
        None => (CityStage::Kampung, Decimal::ZERO, 10, 100, 100, Decimal::ONE),
    };
    let wallet_balance = wallet_repo::balance(&mut *conn, player_id)
        .await?
        .unwrap_or(Decimal::ZERO);
    let facilities: Vec<FacilityView> = city_repo::list_facilities(&mut *conn, player_id)
        .await?
        .into_iter()
        .filter_map(|f| {
            ProjectId::parse(&f.facility_id).map(|id| FacilityView { id, level: f.level })
        })
        .collect();
    let assigned_crews = crew_repo::count_assigned_crews(&mut *conn, player_id).await?;
    let preview = settle_city_economy_tick(
        treasury,
        stage,
        facilities.len() as i64,
        assigned_crews,
        tax_rate_pct,
        welfare,
    );
    let granary_lvl = facilities
        .iter()
        .find(|f| f.id == ProjectId::Granary)
        .map(|f| f.level as i64)
        .unwrap_or(0);
    let yield_bonus_pct = yield_bonus_from_effectiveness(effectiveness_pct) + granary_lvl * 3;
    let running_project = city_repo::get_running_project(&mut *conn, player_id)
        .await?
        .and_then(|r| {
            ProjectId::parse(&r.project_id).map(|project| RunningProjectView {
                id: r.id,
                project,
                ends_at: r.ends_at,
            })
        });
    Ok(CityOverview {
        stage,
        treasury,
        wallet_balance,
        tax_rate_pct,
        welfare,
        effectiveness_pct,
        local_modifier,
        est_tax_income: preview.tax_income,
        est_expense: preview.maintenance + preview.crew_salary,
        assigned_crews,
        yield_bonus_pct,
        facilities,
        running_project,
    })
}

/// Memulai proyek kota. Biaya Bac (dari Player Wallet) dan resource dipotong atomik;
/// jika gagal, seluruh transaksi rollback tanpa mengubah saldo atau inventory.
pub async fn start_project(
    pool: &PgPool,
    player_id: i64,
    project: ProjectId,
    duration_pct: i64,
    idempotency_key: &str,
) -> Result<RunningProjectView, CityError> {
    let mut tx = pool.begin().await?;
    if !player_repo::lock(&mut *tx, player_id).await? {
        return Err(CityError::NotRegistered);
    }
    city_repo::create(&mut *tx, player_id).await?;

    if let Some(running) = city_repo::get_running_project(&mut *tx, player_id).await? {
        let name = ProjectId::parse(&running.project_id)
            .map(|p| p.name().to_string())
            .unwrap_or(running.project_id);
        return Err(CityError::ProjectAlreadyRunning(name));
    }

    let pdef = def(project);
    let stage = stage_for_conn(&mut *tx, player_id).await?;
    if let Err(err) = validate_project_prereqs(&mut *tx, player_id, project, stage).await {
        return Err(err);
    }

    let existing_lvl = city_repo::facility_level(&mut *tx, player_id, project.as_str()).await?;
    if existing_lvl >= pdef.max_level {
        return Err(CityError::AlreadyBuilt(pdef.name));
    }
    let next_level = existing_lvl + 1;

    let cost_bac = project_cost_bac_for_level(pdef, next_level);
    let treasury = city_repo::get(&mut *tx, player_id)
        .await?
        .map(|c| c.treasury)
        .unwrap_or(Decimal::ZERO);
    let have_bac = wallet_repo::balance(&mut *tx, player_id)
        .await?
        .unwrap_or(Decimal::ZERO);
    let total_avail = treasury + have_bac;
    if total_avail < cost_bac {
        return Err(CityError::InsufficientBac { need: cost_bac, have: total_avail });
    }

    for (item_id, need) in pdef.cost_items {
        let total_need = *need * (next_level as i64);
        let have = inventory_repo::quantity_of(&mut *tx, player_id, item_id).await?;
        if have < total_need {
            let name = find_item(item_id).map(|i| i.name).unwrap_or(item_id);
            return Err(CityError::InsufficientItem { name, need: total_need, have });
        }
    }

    let from_treasury = treasury.min(cost_bac);
    let from_wallet = cost_bac - from_treasury;
    let reason = format!("city_project:{}", project.as_str());
    if from_treasury > Decimal::ZERO {
        if let ApplyOutcome::Duplicate = city_repo::apply_treasury(
            &mut *tx,
            player_id,
            -from_treasury,
            &reason,
            &format!("t:{idempotency_key}"),
        )
        .await?
        {
            if let Some(running) = city_repo::get_running_project(&mut *tx, player_id).await? {
                return Ok(RunningProjectView {
                    id: running.id,
                    project,
                    ends_at: running.ends_at,
                });
            }
            return Err(CityError::AlreadyBuilt(pdef.name));
        }
    }
    if from_wallet > Decimal::ZERO {
        let w_key = if from_treasury > Decimal::ZERO {
            format!("w:{idempotency_key}")
        } else {
            idempotency_key.to_string()
        };
        if let ApplyOutcome::Duplicate =
            wallet_repo::apply(&mut *tx, player_id, -from_wallet, &reason, &w_key).await?
        {
            if let Some(running) = city_repo::get_running_project(&mut *tx, player_id).await? {
                return Ok(RunningProjectView {
                    id: running.id,
                    project,
                    ends_at: running.ends_at,
                });
            }
            return Err(CityError::AlreadyBuilt(pdef.name));
        }
    }

    for (item_id, need) in pdef.cost_items {
        let total_need = *need * (next_level as i64);
        inventory_repo::remove_item(&mut *tx, player_id, item_id, total_need).await?;
    }

    let secs = (pdef.duration_secs * duration_pct.max(1) / 100).max(1);
    let (id, ends_at) = city_repo::insert_project(&mut *tx, player_id, project.as_str(), secs).await?;
    scheduler::enqueue(&mut *tx, JOB_CITY_PROJECT_COMPLETE, id, ends_at).await?;
    tx.commit().await?;
    Ok(RunningProjectView { id, project, ends_at })
}

async fn validate_project_prereqs(
    conn: &mut PgConnection,
    player_id: i64,
    project: ProjectId,
    stage: CityStage,
) -> Result<(), CityError> {
    let pdef = def(project);
    if stage < pdef.min_stage {
        return Err(CityError::StageTooLow(pdef.min_stage));
    }
    match project {
        ProjectId::TownHall => {
            if stage != CityStage::Kampung {
                return Err(CityError::StageAlreadyAdvanced);
            }
            let well_lvl =
                city_repo::facility_level(conn, player_id, ProjectId::Well.as_str()).await?;
            let granary_lvl =
                city_repo::facility_level(conn, player_id, ProjectId::Granary.as_str()).await?;
            if well_lvl == 0 || granary_lvl == 0 {
                return Err(CityError::MissingPrerequisites);
            }
        }
        ProjectId::CityHall => {
            if stage != CityStage::Desa {
                return Err(CityError::StageAlreadyAdvanced);
            }
            let mh_lvl =
                city_repo::facility_level(conn, player_id, ProjectId::MarketHall.as_str()).await?;
            if mh_lvl == 0 {
                return Err(CityError::MissingPrerequisites);
            }
        }
        ProjectId::TradePort if stage != CityStage::KotaKecil => {
            return Err(CityError::StageAlreadyAdvanced);
        }
        ProjectId::MetropolisTower if stage != CityStage::KotaDagang => {
            return Err(CityError::StageAlreadyAdvanced);
        }
        _ => {}
    }
    Ok(())
}

async fn quote_conn(
    conn: &mut PgConnection,
    player_id: i64,
    project: ProjectId,
    duration_pct: i64,
    stage: CityStage,
    available_bac: Decimal,
    running_name: Option<&str>,
) -> anyhow::Result<ProjectQuoteView> {
    let pdef = def(project);
    let current_level = city_repo::facility_level(conn, player_id, project.as_str()).await?;
    if current_level >= pdef.max_level {
        return Ok(ProjectQuoteView {
            project,
            current_level,
            next_level: None,
            max_level: pdef.max_level,
            min_stage: pdef.min_stage,
            cost_bac: Decimal::ZERO,
            available_bac,
            duration_secs: 0,
            items: Vec::new(),
            can_start: false,
            block_reason: Some(CityError::AlreadyBuilt(pdef.name).to_string()),
        });
    }

    let next_level = current_level + 1;
    let cost_bac = project_cost_bac_for_level(pdef, next_level);
    let duration_secs = (pdef.duration_secs * duration_pct.max(1) / 100).max(1);
    let mut items = Vec::with_capacity(pdef.cost_items.len());
    let mut missing_item: Option<(&'static str, i64, i64)> = None;
    for (item_id, need) in pdef.cost_items {
        let total_need = *need * (next_level as i64);
        let have = inventory_repo::quantity_of(conn, player_id, item_id).await?;
        if missing_item.is_none() && have < total_need {
            let name = find_item(item_id).map(|i| i.name).unwrap_or(item_id);
            missing_item = Some((name, total_need, have));
        }
        items.push(ProjectQuoteItem { item_id, need: total_need, have });
    }

    let block_reason = if let Some(name) = running_name {
        Some(CityError::ProjectAlreadyRunning(name.to_string()).to_string())
    } else if let Err(err) = validate_project_prereqs(conn, player_id, project, stage).await {
        Some(err.to_string())
    } else if available_bac < cost_bac {
        Some(CityError::InsufficientBac { need: cost_bac, have: available_bac }.to_string())
    } else {
        missing_item.map(|(name, need, have)| {
            CityError::InsufficientItem { name, need, have }.to_string()
        })
    };

    Ok(ProjectQuoteView {
        project,
        current_level,
        next_level: Some(next_level),
        max_level: pdef.max_level,
        min_stage: pdef.min_stage,
        cost_bac,
        available_bac,
        duration_secs,
        items,
        can_start: block_reason.is_none(),
        block_reason,
    })
}

pub async fn quote_all(
    pool: &PgPool,
    player_id: i64,
    duration_pct: i64,
) -> anyhow::Result<Vec<ProjectQuoteView>> {
    let mut conn = pool.acquire().await?;
    let stage = stage_for_conn(&mut *conn, player_id).await?;
    let treasury = city_repo::get(&mut *conn, player_id)
        .await?
        .map(|c| c.treasury)
        .unwrap_or(Decimal::ZERO);
    let wallet = wallet_repo::balance(&mut *conn, player_id)
        .await?
        .unwrap_or(Decimal::ZERO);
    let available_bac = treasury + wallet;
    let running_name = city_repo::get_running_project(&mut *conn, player_id)
        .await?
        .map(|r| {
            ProjectId::parse(&r.project_id)
                .map(|p| p.name().to_string())
                .unwrap_or(r.project_id)
        });
    let mut out = Vec::with_capacity(ProjectId::ALL.len());
    for project in ProjectId::ALL {
        out.push(
            quote_conn(
                &mut *conn,
                player_id,
                project,
                duration_pct,
                stage,
                available_bac,
                running_name.as_deref(),
            )
            .await?,
        );
    }
    Ok(out)
}

pub async fn quote(
    pool: &PgPool,
    player_id: i64,
    project: ProjectId,
    duration_pct: i64,
) -> anyhow::Result<ProjectQuoteView> {
    let mut conn = pool.acquire().await?;
    let stage = stage_for_conn(&mut *conn, player_id).await?;
    let treasury = city_repo::get(&mut *conn, player_id)
        .await?
        .map(|c| c.treasury)
        .unwrap_or(Decimal::ZERO);
    let wallet = wallet_repo::balance(&mut *conn, player_id)
        .await?
        .unwrap_or(Decimal::ZERO);
    let available_bac = treasury + wallet;
    let running_name = city_repo::get_running_project(&mut *conn, player_id)
        .await?
        .map(|r| {
            ProjectId::parse(&r.project_id)
                .map(|p| p.name().to_string())
                .unwrap_or(r.project_id)
        });
    quote_conn(
        &mut *conn,
        player_id,
        project,
        duration_pct,
        stage,
        available_bac,
        running_name.as_deref(),
    )
    .await
}

/// Dipanggil worker di dalam transaksi job. Idempoten: hanya memproses proyek berstatus 'running'.
pub async fn complete_project(
    conn: &mut PgConnection,
    project_run_id: i64,
) -> anyhow::Result<Option<(i64, String)>> {
    let Some(proj) = city_repo::lock_project(conn, project_run_id).await? else {
        return Ok(None);
    };
    if proj.status != "running" {
        return Ok(None);
    }
    let kind = ProjectId::parse(&proj.project_id)
        .ok_or_else(|| anyhow::anyhow!("proyek tidak dikenal: {}", proj.project_id))?;

    city_repo::upsert_facility(conn, proj.player_id, kind.as_str()).await?;
    let stage = stage_for_conn(conn, proj.player_id).await?;
    let next_stage = match (kind, stage) {
        (ProjectId::TownHall, CityStage::Kampung) => Some(CityStage::Desa),
        (ProjectId::CityHall, CityStage::Desa) => Some(CityStage::KotaKecil),
        (ProjectId::TradePort, CityStage::KotaKecil) => Some(CityStage::KotaDagang),
        (ProjectId::MetropolisTower, CityStage::KotaDagang) => Some(CityStage::Metropolis),
        _ => None,
    };
    let mut extra = String::new();
    if let Some(ns) = next_stage {
        city_repo::set_stage(conn, proj.player_id, ns.as_str()).await?;
        let s_name = stage_name(ns, Lang::En);
        extra = format!(" Your city advanced to **{}**!", s_name);
        let flushed =
            crate::application::activities::flush_overflow_for_stage_conn(conn, proj.player_id, ns)
                .await?;
        if !flushed.is_empty() {
            let ups = flushed
                .into_iter()
                .map(|(k, _, nl)| format!("{} -> Lv {nl}", activity_name(k, Lang::En)))
                .collect::<Vec<_>>()
                .join(", ");
            extra.push_str(&format!(" Overflow XP applied ({ups})!"));
        }
    }
    city_repo::mark_project_completed(conn, project_run_id).await?;
    let p_name = project_name(kind, Lang::En);
    let msg = format!("🏗️ Project **{}** completed!{}", p_name, extra);
    Ok(Some((proj.player_id, msg)))
}

/// Memindahkan Bac dari Player Wallet ke City Treasury secara atomik.
pub async fn deposit_to_treasury(
    pool: &PgPool,
    player_id: i64,
    amount: Decimal,
    idempotency_key: &str,
) -> Result<(Decimal, Decimal), CityError> {
    let amount = amount.round_dp(2);
    if amount <= Decimal::ZERO {
        return Err(CityError::InvalidAmount);
    }
    let mut tx = pool.begin().await?;
    if !player_repo::lock(&mut *tx, player_id).await? {
        return Err(CityError::NotRegistered);
    }
    city_repo::create(&mut *tx, player_id).await?;

    let have = wallet_repo::balance(&mut *tx, player_id)
        .await?
        .unwrap_or(Decimal::ZERO);
    if have < amount {
        return Err(CityError::InsufficientBac { need: amount, have });
    }

    let wallet_out = wallet_repo::apply(
        &mut *tx,
        player_id,
        -amount,
        "treasury_deposit",
        &format!("w:{idempotency_key}"),
    )
    .await?;
    let treasury_out = city_repo::apply_treasury(
        &mut *tx,
        player_id,
        amount,
        "player_deposit",
        &format!("t:{idempotency_key}"),
    )
    .await?;

    match (wallet_out, treasury_out) {
        (ApplyOutcome::Applied { balance: w }, ApplyOutcome::Applied { balance: t }) => {
            tx.commit().await?;
            Ok((w, t))
        }
        _ => {
            // Duplicate: ambil saldo terkini tanpa commit perubahan baru
            let w = wallet_repo::balance(&mut *tx, player_id)
                .await?
                .unwrap_or(Decimal::ZERO);
            let t = city_repo::get(&mut *tx, player_id)
                .await?
                .map(|c| c.treasury)
                .unwrap_or(Decimal::ZERO);
            Ok((w, t))
        }
    }
}

pub async fn withdraw_from_treasury(
    pool: &PgPool,
    player_id: i64,
    amount: Decimal,
    idempotency_key: &str,
) -> Result<(Decimal, Decimal), CityError> {
    let amount = amount.round_dp(2);
    if amount <= Decimal::ZERO {
        return Err(CityError::InvalidAmount);
    }
    let mut tx = pool.begin().await?;
    if !player_repo::lock(&mut *tx, player_id).await? {
        return Err(CityError::NotRegistered);
    }
    city_repo::create(&mut *tx, player_id).await?;

    let have = city_repo::get(&mut *tx, player_id)
        .await?
        .map(|c| c.treasury)
        .unwrap_or(Decimal::ZERO);
    if have < amount {
        return Err(CityError::InsufficientTreasury { need: amount, have });
    }

    let treasury_out = city_repo::apply_treasury(
        &mut *tx,
        player_id,
        -amount,
        "player_withdraw",
        &format!("tw:{idempotency_key}"),
    )
    .await?;
    let wallet_out = wallet_repo::apply(
        &mut *tx,
        player_id,
        amount,
        "treasury_withdraw",
        &format!("ww:{idempotency_key}"),
    )
    .await?;

    match (wallet_out, treasury_out) {
        (ApplyOutcome::Applied { balance: w }, ApplyOutcome::Applied { balance: t }) => {
            tx.commit().await?;
            Ok((w, t))
        }
        _ => {
            let w = wallet_repo::balance(&mut *tx, player_id)
                .await?
                .unwrap_or(Decimal::ZERO);
            let t = city_repo::get(&mut *tx, player_id)
                .await?
                .map(|c| c.treasury)
                .unwrap_or(Decimal::ZERO);
            Ok((w, t))
        }
    }
}

pub async fn set_tax_rate(pool: &PgPool, player_id: i64, pct: i32) -> Result<i32, CityError> {
    if !(MIN_TAX_RATE..=MAX_TAX_RATE).contains(&pct) {
        return Err(CityError::InvalidTaxRate);
    }
    let mut tx = pool.begin().await?;
    if !player_repo::lock(&mut *tx, player_id).await? {
        return Err(CityError::NotRegistered);
    }
    city_repo::create(&mut *tx, player_id).await?;
    city_repo::set_tax_rate(&mut *tx, player_id, pct).await?;
    tx.commit().await?;
    Ok(pct)
}

pub async fn effectiveness_for_conn(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<i32> {
    let eff = city_repo::get(conn, player_id)
        .await?
        .map(|c| c.effectiveness_pct)
        .unwrap_or(100);
    Ok(eff)
}

/// Mengembalikan `(extra_yield_pct, extra_efficiency_pct)` dari kota & fasilitas terpasang.
pub async fn activity_bonuses_conn(
    conn: &mut PgConnection,
    player_id: i64,
    kind: ActivityKind,
) -> sqlx::Result<(i64, i64)> {
    let eff_yield = yield_bonus_from_effectiveness(effectiveness_for_conn(conn, player_id).await?);
    let well_yield = if matches!(kind, ActivityKind::Farming | ActivityKind::Gathering)
        && city_repo::facility_level(conn, player_id, ProjectId::Well.as_str()).await? > 0
    {
        10
    } else {
        0
    };
    let granary_lvl =
        city_repo::facility_level(conn, player_id, ProjectId::Granary.as_str()).await? as i64;
    let metro_eff =
        if city_repo::facility_level(conn, player_id, ProjectId::MetropolisTower.as_str()).await?
            > 0
        {
            15
        } else {
            0
        };
    Ok((eff_yield + well_yield + granary_lvl * 3, metro_eff))
}

/// Mengembalikan `(max_concurrent_crafts, order_bonus_pct, craft_efficiency_pct)` dari fasilitas.
pub async fn craft_bonuses_conn(
    conn: &mut PgConnection,
    player_id: i64,
) -> sqlx::Result<(i64, i64, i64)> {
    let has_port =
        city_repo::facility_level(conn, player_id, ProjectId::TradePort.as_str()).await? > 0;
    let has_metro =
        city_repo::facility_level(conn, player_id, ProjectId::MetropolisTower.as_str()).await?
            > 0;
    Ok((
        if has_port { 3 } else { 2 },
        if has_port { 15 } else { 0 },
        if has_metro { 15 } else { 0 },
    ))
}

pub async fn market_hall_bonus_pct_conn(
    conn: &mut PgConnection,
    player_id: i64,
) -> sqlx::Result<i64> {
    let has =
        city_repo::facility_level(conn, player_id, ProjectId::MarketHall.as_str()).await? > 0;
    Ok(if has { 5 } else { 0 })
}

/// Memproses satu siklus pajak warga, maintenance fasilitas, gaji kru, dan welfare semua kota.
pub async fn run_city_economy_tick_conn(
    conn: &mut PgConnection,
    tick_ref: i64,
) -> anyhow::Result<Vec<(i64, String)>> {
    let cities = city_repo::list_all_cities(conn).await?;
    let mut notes = Vec::new();
    for c in cities {
        let stage = CityStage::parse(&c.stage).unwrap_or(CityStage::Kampung);
        let facility_count = city_repo::list_facilities(conn, c.player_id).await?.len() as i64;
        let crew_count = crew_repo::count_assigned_crews(conn, c.player_id).await?;
        let outcome = settle_city_economy_tick(
            c.treasury,
            stage,
            facility_count,
            crew_count,
            c.tax_rate_pct,
            c.welfare,
        );
        if outcome.treasury_delta != Decimal::ZERO {
            let key = format!("city_tick:{tick_ref}:{}", c.player_id);
            let _ = city_repo::apply_treasury(
                conn,
                c.player_id,
                outcome.treasury_delta,
                "city_economy_tick",
                &key,
            )
            .await?;
        }
        let local_mod = local_modifier_from_city(stage, outcome.new_welfare);
        city_repo::update_economy_state(
            conn,
            c.player_id,
            outcome.new_welfare,
            outcome.new_effectiveness_pct,
            local_mod,
        )
        .await?;
        if outcome.shortfall {
            let msg = format!(
                "⚠️ City treasury is empty! Welfare dropped to **{}%** (effectiveness **{}%**). Deposit Bac into Treasury in the **City** menu.",
                outcome.new_welfare, outcome.new_effectiveness_pct
            );
            notes.push((c.player_id, msg));
        }
    }
    Ok(notes)
}

pub async fn handle_city_economy_tick_job(
    conn: &mut PgConnection,
    tick_ref: i64,
    duration_pct: i64,
) -> anyhow::Result<Vec<(i64, String)>> {
    let notes = run_city_economy_tick_conn(conn, tick_ref).await?;
    let secs = (CITY_ECONOMY_TICK_SECS * duration_pct.max(1) / 100).max(5);
    let next_ref = city_repo::next_city_economy_tick_ref(conn).await?;
    scheduler::enqueue(
        conn,
        JOB_CITY_ECONOMY_TICK,
        next_ref,
        Utc::now() + Duration::seconds(secs),
    )
    .await?;
    Ok(notes)
}

pub async fn ensure_city_economy_tick_scheduled(
    pool: &PgPool,
    duration_pct: i64,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;
    if !city_repo::has_pending_city_economy_tick(&mut *conn).await? {
        let secs = (CITY_ECONOMY_TICK_SECS * duration_pct.max(1) / 100).max(5);
        let next_ref = city_repo::next_city_economy_tick_ref(&mut *conn).await?;
        scheduler::enqueue(
            &mut *conn,
            JOB_CITY_ECONOMY_TICK,
            next_ref,
            Utc::now() + Duration::seconds(secs),
        )
        .await?;
    }
    Ok(())
}
