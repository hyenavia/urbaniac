use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgConnection;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Giveaway {
    pub id: i64,
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: i64,
    pub host_id: i64,
    pub kind: String,
    pub prize: String,
    pub cash_amount: Decimal,
    pub item_id: Option<String>,
    pub item_qty: i64,
    pub winner_count: i32,
    pub ends_at: DateTime<Utc>,
    pub status: String,
    pub winner_ids: Vec<i64>,
}

const COLS: &str = "id, guild_id, channel_id, message_id, host_id, kind, prize, cash_amount, item_id, item_qty, winner_count, ends_at, status, winner_ids";

#[allow(clippy::too_many_arguments)]
pub async fn create_giveaway(
    conn: &mut PgConnection,
    guild_id: i64,
    channel_id: i64,
    host_id: i64,
    kind: &str,
    prize: &str,
    cash_amount: Decimal,
    item_id: Option<&str>,
    item_qty: i64,
    winner_count: i32,
    ends_at: DateTime<Utc>,
) -> sqlx::Result<Giveaway> {
    let sql = format!(
        "INSERT INTO giveaways (guild_id, channel_id, host_id, kind, prize, cash_amount, item_id, item_qty, winner_count, ends_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING {COLS}"
    );
    sqlx::query_as::<_, Giveaway>(&sql)
        .bind(guild_id)
        .bind(channel_id)
        .bind(host_id)
        .bind(kind)
        .bind(prize)
        .bind(cash_amount)
        .bind(item_id)
        .bind(item_qty)
        .bind(winner_count)
        .bind(ends_at)
        .fetch_one(&mut *conn)
        .await
}

pub async fn set_message_id(
    conn: &mut PgConnection,
    id: i64,
    message_id: i64,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE giveaways SET message_id = $2 WHERE id = $1")
        .bind(id)
        .bind(message_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn delete_unposted(conn: &mut PgConnection, id: i64) -> sqlx::Result<()> {
    sqlx::query(
        "DELETE FROM scheduled_jobs
         WHERE kind = 'giveaway_end' AND ref_id = $1 AND status = 'pending'",
    )
    .bind(id)
    .execute(&mut *conn)
    .await?;

    sqlx::query("DELETE FROM giveaways WHERE id = $1 AND message_id = 0")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn get_giveaway(conn: &mut PgConnection, id: i64) -> sqlx::Result<Option<Giveaway>> {
    let sql = format!("SELECT {COLS} FROM giveaways WHERE id = $1");
    sqlx::query_as::<_, Giveaway>(&sql)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
}

pub async fn lock_giveaway(conn: &mut PgConnection, id: i64) -> sqlx::Result<Option<Giveaway>> {
    let sql = format!("SELECT {COLS} FROM giveaways WHERE id = $1 FOR UPDATE");
    sqlx::query_as::<_, Giveaway>(&sql)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
}

pub async fn add_entry(
    conn: &mut PgConnection,
    giveaway_id: i64,
    user_id: i64,
) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "INSERT INTO giveaway_entries (giveaway_id, user_id) VALUES ($1, $2)
         ON CONFLICT (giveaway_id, user_id) DO NOTHING",
    )
    .bind(giveaway_id)
    .bind(user_id)
    .execute(&mut *conn)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn count_entries(conn: &mut PgConnection, giveaway_id: i64) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM giveaway_entries WHERE giveaway_id = $1")
        .bind(giveaway_id)
        .fetch_one(&mut *conn)
        .await
}

pub async fn list_entries(conn: &mut PgConnection, giveaway_id: i64) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar::<_, i64>(
        "SELECT user_id FROM giveaway_entries WHERE giveaway_id = $1 ORDER BY entered_at, user_id",
    )
    .bind(giveaway_id)
    .fetch_all(&mut *conn)
    .await
}

pub async fn mark_ended(
    conn: &mut PgConnection,
    id: i64,
    winner_ids: &[i64],
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE giveaways
         SET status = 'ended', winner_ids = $2, ended_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(winner_ids)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn update_winners(
    conn: &mut PgConnection,
    id: i64,
    winner_ids: &[i64],
) -> sqlx::Result<()> {
    sqlx::query("UPDATE giveaways SET winner_ids = $2 WHERE id = $1")
        .bind(id)
        .bind(winner_ids)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
