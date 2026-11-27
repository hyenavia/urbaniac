use crate::infrastructure::repositories::wallet_repo::{ApplyOutcome, WalletError};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgConnection;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CityRow {
    pub player_id: i64,
    pub stage: String,
    pub treasury: Decimal,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FacilityRow {
    pub facility_id: String,
    pub level: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProjectRow {
    pub id: i64,
    pub player_id: i64,
    pub project_id: String,
    pub status: String,
    pub ends_at: DateTime<Utc>,
}

pub async fn create(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO cities (player_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(player_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn get(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<Option<CityRow>> {
    sqlx::query_as::<_, CityRow>(
        "SELECT player_id, stage, treasury FROM cities WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn get_stage(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar::<_, String>("SELECT stage FROM cities WHERE player_id = $1")
        .bind(player_id)
        .fetch_optional(&mut *conn)
        .await
}

pub async fn set_stage(conn: &mut PgConnection, player_id: i64, stage: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE cities SET stage = $2, updated_at = now() WHERE player_id = $1")
        .bind(player_id)
        .bind(stage)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Mengubah saldo City Treasury dengan ledger & idempotency key. Terpisah dari Player Wallet.
pub async fn apply_treasury(
    conn: &mut PgConnection,
    player_id: i64,
    delta: Decimal,
    reason: &str,
    idempotency_key: &str,
) -> Result<ApplyOutcome, WalletError> {
    let delta = delta.round_dp(2);

    let current: Option<Decimal> =
        sqlx::query_scalar("SELECT treasury FROM cities WHERE player_id = $1 FOR UPDATE")
            .bind(player_id)
            .fetch_optional(&mut *conn)
            .await?;
    let current = current.ok_or(WalletError::NotFound)?;

    let exists: Option<i64> =
        sqlx::query_scalar("SELECT id FROM city_treasury_ledger WHERE idempotency_key = $1")
            .bind(idempotency_key)
            .fetch_optional(&mut *conn)
            .await?;
    if exists.is_some() {
        return Ok(ApplyOutcome::Duplicate);
    }

    let new_balance = current + delta;
    if new_balance < Decimal::ZERO {
        return Err(WalletError::InsufficientFunds);
    }

    sqlx::query("UPDATE cities SET treasury = $2, updated_at = now() WHERE player_id = $1")
        .bind(player_id)
        .bind(new_balance)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO city_treasury_ledger (player_id, amount, balance_after, reason, idempotency_key)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(player_id)
    .bind(delta)
    .bind(new_balance)
    .bind(reason)
    .bind(idempotency_key)
    .execute(&mut *conn)
    .await?;

    Ok(ApplyOutcome::Applied { balance: new_balance })
}

pub async fn facility_level(
    conn: &mut PgConnection,
    player_id: i64,
    facility_id: &str,
) -> sqlx::Result<i32> {
    let lvl = sqlx::query_scalar::<_, i32>(
        "SELECT level FROM city_facilities WHERE player_id = $1 AND facility_id = $2",
    )
    .bind(player_id)
    .bind(facility_id)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(lvl.unwrap_or(0))
}

pub async fn list_facilities(
    conn: &mut PgConnection,
    player_id: i64,
) -> sqlx::Result<Vec<FacilityRow>> {
    sqlx::query_as::<_, FacilityRow>(
        "SELECT facility_id, level FROM city_facilities WHERE player_id = $1 ORDER BY facility_id",
    )
    .bind(player_id)
    .fetch_all(&mut *conn)
    .await
}

pub async fn upsert_facility(
    conn: &mut PgConnection,
    player_id: i64,
    facility_id: &str,
) -> sqlx::Result<i32> {
    sqlx::query_scalar::<_, i32>(
        "INSERT INTO city_facilities (player_id, facility_id, level)
         VALUES ($1, $2, 1)
         ON CONFLICT (player_id, facility_id) DO UPDATE
         SET level = city_facilities.level + 1, updated_at = now()
         RETURNING level",
    )
    .bind(player_id)
    .bind(facility_id)
    .fetch_one(&mut *conn)
    .await
}

pub async fn get_running_project(
    conn: &mut PgConnection,
    player_id: i64,
) -> sqlx::Result<Option<ProjectRow>> {
    sqlx::query_as::<_, ProjectRow>(
        "SELECT id, player_id, project_id, status, ends_at
         FROM city_projects
         WHERE player_id = $1 AND status = 'running'
         LIMIT 1",
    )
    .bind(player_id)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn insert_project(
    conn: &mut PgConnection,
    player_id: i64,
    project_id: &str,
    duration_secs: i64,
) -> sqlx::Result<(i64, DateTime<Utc>)> {
    sqlx::query_as::<_, (i64, DateTime<Utc>)>(
        "INSERT INTO city_projects (player_id, project_id, status, ends_at)
         VALUES ($1, $2, 'running', now() + make_interval(secs => $3::double precision))
         RETURNING id, ends_at",
    )
    .bind(player_id)
    .bind(project_id)
    .bind(duration_secs as f64)
    .fetch_one(&mut *conn)
    .await
}

pub async fn lock_project(conn: &mut PgConnection, id: i64) -> sqlx::Result<Option<ProjectRow>> {
    sqlx::query_as::<_, ProjectRow>(
        "SELECT id, player_id, project_id, status, ends_at
         FROM city_projects
         WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn mark_project_completed(conn: &mut PgConnection, id: i64) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE city_projects SET status = 'completed', completed_at = now() WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
