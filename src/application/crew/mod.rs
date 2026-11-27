use crate::application::city;
use crate::domain::activities::ActivityKind;
use crate::domain::city::CityStage;
use crate::domain::crew::{
    crew_bonus, parse_rarity, rarity_str, recruit_cost, roll_recruit_for_stage, upgrade_cost,
    CrewBonus, MAX_CREW_LEVEL, MAX_CREW_SLOTS,
};
use crate::domain::resources::Rarity;
use crate::infrastructure::repositories::wallet_repo::{self, ApplyOutcome, WalletError};
use crate::infrastructure::repositories::{crew_repo, player_repo};
use rand::{rngs::StdRng, SeedableRng};
use rust_decimal::Decimal;
use sqlx::{PgConnection, PgPool};

#[derive(Debug, thiserror::Error)]
pub enum CrewError {
    #[error("profil belum ada, jalankan /start")]
    NotRegistered,
    #[error("slot kru sudah penuh (maksimal {0} kru)")]
    CrewFull(i64),
    #[error("kru tidak ditemukan")]
    NotFound,
    #[error("kru sudah di level maksimal ({0})")]
    MaxLevel(i32),
    #[error("aktivitas ini terbuka di stage {}", .0.name())]
    ActivityLocked(CityStage),
    #[error("saldo Bac tidak cukup (butuh {need}, saldo {have})")]
    InsufficientBac { need: Decimal, have: Decimal },
    #[error(transparent)]
    Wallet(#[from] WalletError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

pub struct CrewView {
    pub id: i64,
    pub name: String,
    pub rarity: Rarity,
    pub specialty: ActivityKind,
    pub level: i32,
    pub assigned: Option<ActivityKind>,
    pub auto_enabled: bool,
    pub bonus: CrewBonus,
    pub upgrade_cost: Option<Decimal>,
}

fn to_view(r: crew_repo::CrewRow) -> CrewView {
    let rarity = parse_rarity(&r.rarity);
    let specialty = ActivityKind::parse(&r.specialty).unwrap_or(ActivityKind::Farming);
    let assigned = r.assigned_activity.as_deref().and_then(ActivityKind::parse);
    let match_spec = assigned.map(|a| a == specialty).unwrap_or(true);
    let bonus = crew_bonus(rarity, r.level, match_spec);
    let upg = if r.level >= MAX_CREW_LEVEL {
        None
    } else {
        Some(upgrade_cost(r.level, rarity))
    };
    CrewView {
        id: r.id,
        name: r.name,
        rarity,
        specialty,
        level: r.level,
        assigned,
        auto_enabled: r.auto_enabled,
        bonus,
        upgrade_cost: upg,
    }
}

pub async fn list(pool: &PgPool, player_id: i64) -> anyhow::Result<Vec<CrewView>> {
    let mut conn = pool.acquire().await?;
    Ok(crew_repo::list_crews(&mut *conn, player_id)
        .await?
        .into_iter()
        .map(to_view)
        .collect())
}

/// Mengambil bonus & status otomasi kru yang ditugaskan pada aktivitas tertentu.
pub async fn bonus_for_activity_conn(
    conn: &mut PgConnection,
    player_id: i64,
    kind: ActivityKind,
) -> sqlx::Result<(CrewBonus, bool)> {
    let Some(r) = crew_repo::get_assigned_for_activity(conn, player_id, kind.as_str()).await? else {
        return Ok((CrewBonus::ZERO, false));
    };
    let rarity = parse_rarity(&r.rarity);
    let spec = ActivityKind::parse(&r.specialty).unwrap_or(ActivityKind::Farming);
    let bonus = crew_bonus(rarity, r.level, spec == kind);
    Ok((bonus, r.auto_enabled))
}

pub async fn recruit(
    pool: &PgPool,
    player_id: i64,
    idempotency_key: &str,
) -> Result<CrewView, CrewError> {
    let mut tx = pool.begin().await?;
    if !player_repo::lock(&mut *tx, player_id).await? {
        return Err(CrewError::NotRegistered);
    }
    if crew_repo::count_crews(&mut *tx, player_id).await? >= MAX_CREW_SLOTS {
        return Err(CrewError::CrewFull(MAX_CREW_SLOTS));
    }
    let cost = recruit_cost();
    let have = wallet_repo::balance(&mut *tx, player_id)
        .await?
        .unwrap_or(Decimal::ZERO);
    if have < cost {
        return Err(CrewError::InsufficientBac { need: cost, have });
    }
    match wallet_repo::apply(&mut *tx, player_id, -cost, "crew_recruit", idempotency_key).await? {
        ApplyOutcome::Duplicate => {
            let list = crew_repo::list_crews(&mut *tx, player_id).await?;
            return list.into_iter().last().map(to_view).ok_or(CrewError::NotFound);
        }
        ApplyOutcome::Applied { .. } => {}
    }
    let stage = city::stage_for_conn(&mut *tx, player_id).await?;
    let mut rng = StdRng::from_entropy();
    let (name, rarity, spec) = roll_recruit_for_stage(stage, &mut rng);
    let row = crew_repo::insert_crew(&mut *tx, player_id, name, rarity_str(rarity), spec.as_str())
        .await?;
    tx.commit().await?;
    Ok(to_view(row))
}

pub async fn upgrade(
    pool: &PgPool,
    player_id: i64,
    crew_id: i64,
    idempotency_key: &str,
) -> Result<CrewView, CrewError> {
    let mut tx = pool.begin().await?;
    if !player_repo::lock(&mut *tx, player_id).await? {
        return Err(CrewError::NotRegistered);
    }
    let Some(row) = crew_repo::lock_crew(&mut *tx, crew_id).await? else {
        return Err(CrewError::NotFound);
    };
    if row.player_id != player_id {
        return Err(CrewError::NotFound);
    }
    if row.level >= MAX_CREW_LEVEL {
        return Err(CrewError::MaxLevel(MAX_CREW_LEVEL));
    }
    let rarity = parse_rarity(&row.rarity);
    let cost = upgrade_cost(row.level, rarity);
    let have = wallet_repo::balance(&mut *tx, player_id)
        .await?
        .unwrap_or(Decimal::ZERO);
    if have < cost {
        return Err(CrewError::InsufficientBac { need: cost, have });
    }
    match wallet_repo::apply(
        &mut *tx,
        player_id,
        -cost,
        &format!("crew_upgrade:{}", row.id),
        idempotency_key,
    )
    .await?
    {
        ApplyOutcome::Duplicate => return Ok(to_view(row)),
        ApplyOutcome::Applied { .. } => {}
    }
    let new_lvl = row.level + 1;
    crew_repo::set_level(&mut *tx, crew_id, new_lvl).await?;
    let updated = crew_repo::lock_crew(&mut *tx, crew_id)
        .await?
        .ok_or(CrewError::NotFound)?;
    tx.commit().await?;
    Ok(to_view(updated))
}

pub async fn assign(
    pool: &PgPool,
    player_id: i64,
    crew_id: i64,
    activity: Option<ActivityKind>,
    auto_enabled: bool,
) -> Result<CrewView, CrewError> {
    let mut tx = pool.begin().await?;
    if !player_repo::lock(&mut *tx, player_id).await? {
        return Err(CrewError::NotRegistered);
    }
    let Some(row) = crew_repo::lock_crew(&mut *tx, crew_id).await? else {
        return Err(CrewError::NotFound);
    };
    if row.player_id != player_id {
        return Err(CrewError::NotFound);
    }
    if let Some(act) = activity {
        let stage = city::stage_for_conn(&mut *tx, player_id).await?;
        if stage < act.min_stage() {
            return Err(CrewError::ActivityLocked(act.min_stage()));
        }
        crew_repo::clear_activity_assignment(&mut *tx, player_id, act.as_str()).await?;
        crew_repo::set_assignment(&mut *tx, crew_id, Some(act.as_str()), auto_enabled).await?;
    } else {
        crew_repo::set_assignment(&mut *tx, crew_id, None, false).await?;
    }
    let updated = crew_repo::lock_crew(&mut *tx, crew_id)
        .await?
        .ok_or(CrewError::NotFound)?;
    tx.commit().await?;
    Ok(to_view(updated))
}
