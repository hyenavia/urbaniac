use crate::domain::progression::XpState;
use chrono::{DateTime, Utc};
use sqlx::PgConnection;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RunRow {
    pub id: i64,
    pub player_id: i64,
    pub activity: String,
    pub status: String,
    pub ends_at: DateTime<Utc>,
}

pub async fn count_occupied(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_runs WHERE player_id = $1 AND status IN ('running', 'ready')",
    )
    .bind(player_id)
    .fetch_one(&mut *conn)
    .await
}

pub async fn insert_run(
    conn: &mut PgConnection,
    player_id: i64,
    activity: &str,
    duration_secs: i64,
) -> sqlx::Result<(i64, DateTime<Utc>)> {
    sqlx::query_as::<_, (i64, DateTime<Utc>)>(
        "INSERT INTO activity_runs (player_id, activity, status, ends_at)
         VALUES ($1, $2, 'running', now() + make_interval(secs => $3))
         RETURNING id, ends_at",
    )
    .bind(player_id)
    .bind(activity)
    .bind(duration_secs as f64)
    .fetch_one(&mut *conn)
    .await
}

pub async fn list_active(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<Vec<RunRow>> {
    sqlx::query_as::<_, RunRow>(
        "SELECT id, player_id, activity, status, ends_at FROM activity_runs
         WHERE player_id = $1 AND status IN ('running', 'ready') ORDER BY id",
    )
    .bind(player_id)
    .fetch_all(&mut *conn)
    .await
}

pub async fn list_ready_ids(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar("SELECT id FROM activity_runs WHERE player_id = $1 AND status = 'ready' ORDER BY id")
        .bind(player_id)
        .fetch_all(&mut *conn)
        .await
}

pub async fn lock_run(conn: &mut PgConnection, run_id: i64) -> sqlx::Result<Option<RunRow>> {
    sqlx::query_as::<_, RunRow>(
        "SELECT id, player_id, activity, status, ends_at FROM activity_runs WHERE id = $1 FOR UPDATE",
    )
    .bind(run_id)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn mark_ready(conn: &mut PgConnection, run_id: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE activity_runs SET status = 'ready', completed_at = now() WHERE id = $1")
        .bind(run_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn mark_claimed(conn: &mut PgConnection, run_id: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE activity_runs SET status = 'claimed', claimed_at = now() WHERE id = $1")
        .bind(run_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn insert_reward(conn: &mut PgConnection, run_id: i64, item_id: &str, qty: i64) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO activity_run_rewards (run_id, item_id, quantity) VALUES ($1, $2, $3)")
        .bind(run_id)
        .bind(item_id)
        .bind(qty)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn list_rewards(conn: &mut PgConnection, run_id: i64) -> sqlx::Result<Vec<(String, i64)>> {
    sqlx::query_as::<_, (String, i64)>(
        "SELECT item_id, quantity FROM activity_run_rewards WHERE run_id = $1 ORDER BY item_id",
    )
    .bind(run_id)
    .fetch_all(&mut *conn)
    .await
}

/// Membaca progres tanpa mengunci. Belum ada baris = level 1.
pub async fn get_progress(conn: &mut PgConnection, player_id: i64, activity: &str) -> sqlx::Result<XpState> {
    let row: Option<(i32, i64, i64)> = sqlx::query_as(
        "SELECT level, xp, overflow_xp FROM player_activities WHERE player_id = $1 AND activity = $2",
    )
    .bind(player_id)
    .bind(activity)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row
        .map(|(level, xp, overflow)| XpState { level, xp, overflow })
        .unwrap_or(XpState::START))
}

/// Membuat baris jika belum ada, lalu mengunci dan membacanya.
pub async fn lock_progress(conn: &mut PgConnection, player_id: i64, activity: &str) -> sqlx::Result<XpState> {
    sqlx::query("INSERT INTO player_activities (player_id, activity) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(player_id)
        .bind(activity)
        .execute(&mut *conn)
        .await?;
    let (level, xp, overflow): (i32, i64, i64) = sqlx::query_as(
        "SELECT level, xp, overflow_xp FROM player_activities
         WHERE player_id = $1 AND activity = $2 FOR UPDATE",
    )
    .bind(player_id)
    .bind(activity)
    .fetch_one(&mut *conn)
    .await?;
    Ok(XpState { level, xp, overflow })
}

pub async fn save_progress(
    conn: &mut PgConnection,
    player_id: i64,
    activity: &str,
    s: XpState,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE player_activities SET level = $3, xp = $4, overflow_xp = $5
         WHERE player_id = $1 AND activity = $2",
    )
    .bind(player_id)
    .bind(activity)
    .bind(s.level)
    .bind(s.xp)
    .bind(s.overflow)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn list_player_activities(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar::<_, String>(
        "SELECT activity FROM player_activities WHERE player_id = $1 ORDER BY activity",
    )
    .bind(player_id)
    .fetch_all(&mut *conn)
    .await
}
