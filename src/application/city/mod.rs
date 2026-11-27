use crate::domain::city::{def, project_cost_bac, CityStage, ProjectId};
use crate::domain::resources::find_item;
use crate::infrastructure::repositories::wallet_repo::{self, ApplyOutcome, WalletError};
use crate::infrastructure::repositories::{city_repo, inventory_repo, player_repo};
use crate::infrastructure::scheduler;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{PgConnection, PgPool};

pub const JOB_CITY_PROJECT_COMPLETE: &str = "city_project_complete";

#[derive(Debug, thiserror::Error)]
pub enum CityError {
    #[error("profil belum ada, jalankan /start")]
    NotRegistered,
    #[error("proyek {0} sudah berjalan")]
    ProjectAlreadyRunning(String),
    #[error("fasilitas {0} sudah dibangun")]
    AlreadyBuilt(&'static str),
    #[error("Balai Kampung membutuhkan Sumur dan Lumbung terlebih dahulu")]
    MissingPrerequisites,
    #[error("Balai Kampung hanya untuk stage Kampung")]
    StageAlreadyAdvanced,
    #[error("saldo Bac tidak cukup (butuh {need}, saldo {have})")]
    InsufficientBac { need: Decimal, have: Decimal },
    #[error("resource {name} tidak cukup (butuh {need}, punya {have})")]
    InsufficientItem { name: &'static str, need: i64, have: i64 },
    #[error("jumlah setoran harus lebih dari 0")]
    InvalidAmount,
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
    let (stage, treasury) = match city {
        Some(c) => (
            CityStage::parse(&c.stage).unwrap_or(CityStage::Kampung),
            c.treasury,
        ),
        None => (CityStage::Kampung, Decimal::ZERO),
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
    let existing_lvl = city_repo::facility_level(&mut *tx, player_id, project.as_str()).await?;
    if existing_lvl > 0 {
        return Err(CityError::AlreadyBuilt(pdef.name));
    }

    if project == ProjectId::TownHall {
        let stage = stage_for_conn(&mut *tx, player_id).await?;
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

    let cost_bac = project_cost_bac(pdef);
    let have_bac = wallet_repo::balance(&mut *tx, player_id)
        .await?
        .unwrap_or(Decimal::ZERO);
    if have_bac < cost_bac {
        return Err(CityError::InsufficientBac { need: cost_bac, have: have_bac });
    }

    for (item_id, need) in pdef.cost_items {
        let have = inventory_repo::quantity_of(&mut *tx, player_id, item_id).await?;
        if have < *need {
            let name = find_item(item_id).map(|i| i.name).unwrap_or(item_id);
            return Err(CityError::InsufficientItem { name, need: *need, have });
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
        inventory_repo::remove_item(&mut *tx, player_id, item_id, *need).await?;
    }

    let secs = (pdef.duration_secs * duration_pct.max(1) / 100).max(1);
    let (id, ends_at) = city_repo::insert_project(&mut *tx, player_id, project.as_str(), secs).await?;
    scheduler::enqueue(&mut *tx, JOB_CITY_PROJECT_COMPLETE, id, ends_at).await?;
    tx.commit().await?;
    Ok(RunningProjectView { id, project, ends_at })
}

/// Dipanggil worker di dalam transaksi job. Idempoten: hanya memproses proyek berstatus 'running'.
pub async fn complete_project(conn: &mut PgConnection, project_run_id: i64) -> anyhow::Result<()> {
    let Some(proj) = city_repo::lock_project(conn, project_run_id).await? else {
        return Ok(());
    };
    if proj.status != "running" {
        return Ok(());
    }
    let kind = ProjectId::parse(&proj.project_id)
        .ok_or_else(|| anyhow::anyhow!("proyek tidak dikenal: {}", proj.project_id))?;

    city_repo::upsert_facility(conn, proj.player_id, kind.as_str()).await?;
    if kind == ProjectId::TownHall {
        let stage = stage_for_conn(conn, proj.player_id).await?;
        if stage == CityStage::Kampung {
            city_repo::set_stage(conn, proj.player_id, CityStage::Desa.as_str()).await?;
        }
    }
    city_repo::mark_project_completed(conn, project_run_id).await?;
    Ok(())
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
