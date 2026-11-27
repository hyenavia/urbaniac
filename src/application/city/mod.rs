use crate::domain::city::{
    def, local_modifier_from_city, project_cost_bac_for_level, settle_city_economy_tick, CityStage,
    ProjectId, CITY_ECONOMY_TICK_SECS, MAX_TAX_RATE, MIN_TAX_RATE,
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
    #[error("profil belum ada, jalankan /start")]
    NotRegistered,
    #[error("proyek {0} sudah berjalan")]
    ProjectAlreadyRunning(String),
    #[error("fasilitas {0} sudah mencapai level maksimal")]
    AlreadyBuilt(&'static str),
    #[error("proyek ini membutuhkan stage kota {}", .0.name())]
    StageTooLow(CityStage),
    #[error("prasyarat fasilitas belum terpenuhi")]
    MissingPrerequisites,
    #[error("proyek kenaikan stage ini sudah terlewati")]
    StageAlreadyAdvanced,
    #[error("saldo Bac tidak cukup (butuh {need}, saldo {have})")]
    InsufficientBac { need: Decimal, have: Decimal },
    #[error("resource {name} tidak cukup (butuh {need}, punya {have})")]
    InsufficientItem { name: &'static str, need: i64, have: i64 },
    #[error("jumlah setoran harus lebih dari 0")]
    InvalidAmount,
    #[error("tarif pajak harus {MIN_TAX_RATE}%–{MAX_TAX_RATE}%")]
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
    pub facilities: Vec<FacilityView>,
    pub running_project: Option<RunningProjectView>,
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
    let facilities = city_repo::list_facilities(&mut *conn, player_id)
        .await?
        .into_iter()
        .filter_map(|f| {
            ProjectId::parse(&f.facility_id).map(|id| FacilityView { id, level: f.level })
        })
        .collect();
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
    if stage < pdef.min_stage {
        return Err(CityError::StageTooLow(pdef.min_stage));
    }

    let existing_lvl = city_repo::facility_level(&mut *tx, player_id, project.as_str()).await?;
    if existing_lvl >= pdef.max_level {
        return Err(CityError::AlreadyBuilt(pdef.name));
    }
    let next_level = existing_lvl + 1;

    match project {
        ProjectId::TownHall => {
            if stage != CityStage::Kampung {
                return Err(CityError::StageAlreadyAdvanced);
            }
            let well_lvl =
                city_repo::facility_level(&mut *tx, player_id, ProjectId::Well.as_str()).await?;
            let granary_lvl =
                city_repo::facility_level(&mut *tx, player_id, ProjectId::Granary.as_str()).await?;
            if well_lvl == 0 || granary_lvl == 0 {
                return Err(CityError::MissingPrerequisites);
            }
        }
        ProjectId::CityHall => {
            if stage != CityStage::Desa {
                return Err(CityError::StageAlreadyAdvanced);
            }
            let mh_lvl =
                city_repo::facility_level(&mut *tx, player_id, ProjectId::MarketHall.as_str())
                    .await?;
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

    let cost_bac = project_cost_bac_for_level(pdef, next_level);
    let have_bac = wallet_repo::balance(&mut *tx, player_id)
        .await?
        .unwrap_or(Decimal::ZERO);
    if have_bac < cost_bac {
        return Err(CityError::InsufficientBac { need: cost_bac, have: have_bac });
    }

    for (item_id, need) in pdef.cost_items {
        let total_need = *need * (next_level as i64);
        let have = inventory_repo::quantity_of(&mut *tx, player_id, item_id).await?;
        if have < total_need {
            let name = find_item(item_id).map(|i| i.name).unwrap_or(item_id);
            return Err(CityError::InsufficientItem { name, need: total_need, have });
        }
    }

    let reason = format!("city_project:{}", project.as_str());
    match wallet_repo::apply(&mut *tx, player_id, -cost_bac, &reason, idempotency_key).await? {
        ApplyOutcome::Duplicate => {
            if let Some(running) = city_repo::get_running_project(&mut *tx, player_id).await? {
                return Ok(RunningProjectView {
                    id: running.id,
                    project,
                    ends_at: running.ends_at,
                });
            }
            return Err(CityError::AlreadyBuilt(pdef.name));
        }
        ApplyOutcome::Applied { .. } => {}
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
    let lang = player_repo::get_lang(conn, proj.player_id)
        .await
        .unwrap_or(Lang::En);
    let mut extra = String::new();
    if let Some(ns) = next_stage {
        city_repo::set_stage(conn, proj.player_id, ns.as_str()).await?;
        let s_name = stage_name(ns, lang);
        extra = match lang {
            Lang::En => format!(" Your city advanced to **{}**!", s_name),
            Lang::Id => format!(" Kotamu naik ke stage **{}**!", s_name),
        };
        let flushed =
            crate::application::activities::flush_overflow_for_stage_conn(conn, proj.player_id, ns)
                .await?;
        if !flushed.is_empty() {
            let ups = flushed
                .into_iter()
                .map(|(k, _, nl)| format!("{} -> Lv {nl}", activity_name(k, lang)))
                .collect::<Vec<_>>()
                .join(", ");
            match lang {
                Lang::En => extra.push_str(&format!(" Overflow XP applied ({ups})!")),
                Lang::Id => extra.push_str(&format!(" XP overflow diterapkan ({ups})!")),
            }
        }
    }
    city_repo::mark_project_completed(conn, project_run_id).await?;
    let p_name = project_name(kind, lang);
    let msg = match lang {
        Lang::En => format!("🏗️ Project **{}** completed!{}", p_name, extra),
        Lang::Id => format!("🏗️ Proyek **{}** selesai!{}", p_name, extra),
    };
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
        let crew_count = crew_repo::count_crews(conn, c.player_id).await?;
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
            let lang = player_repo::get_lang(conn, c.player_id)
                .await
                .unwrap_or(Lang::En);
            let msg = match lang {
                Lang::En => format!(
                    "⚠️ City treasury is empty! Welfare dropped to **{}%** (effectiveness **{}%**). Deposit Bac into Treasury in the **City** menu.",
                    outcome.new_welfare, outcome.new_effectiveness_pct
                ),
                Lang::Id => format!(
                    "⚠️ Treasury kota kosong! Welfare turun ke **{}%** (efektivitas **{}%**). Setor Bac ke Treasury di menu **Kota**.",
                    outcome.new_welfare, outcome.new_effectiveness_pct
                ),
            };
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
