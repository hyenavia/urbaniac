use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgConnection;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WelcomeConfig {
    pub guild_id: i64,
    pub channel_id: i64,
    pub auto_role_id: Option<i64>,
    pub title: String,
    pub message: String,
    pub enabled: bool,
}

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
    pub winner_count: i32,
    pub ends_at: DateTime<Utc>,
    pub status: String,
    pub winner_ids: Vec<i64>,
}

pub async fn upsert_welcome(conn: &mut PgConnection, cfg: &WelcomeConfig) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO guild_welcome_configs (guild_id, channel_id, auto_role_id, title, message, enabled, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, now())
         ON CONFLICT (guild_id) DO UPDATE
         SET channel_id = EXCLUDED.channel_id,
             auto_role_id = EXCLUDED.auto_role_id,
             title = EXCLUDED.title,
             message = EXCLUDED.message,
             enabled = EXCLUDED.enabled,
             updated_at = now()",
    )
    .bind(cfg.guild_id)
    .bind(cfg.channel_id)
    .bind(cfg.auto_role_id)
    .bind(&cfg.title)
    .bind(&cfg.message)
    .bind(cfg.enabled)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn set_welcome_enabled(
    conn: &mut PgConnection,
    guild_id: i64,
    enabled: bool,
) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "UPDATE guild_welcome_configs SET enabled = $2, updated_at = now() WHERE guild_id = $1",
    )
    .bind(guild_id)
    .bind(enabled)
    .execute(&mut *conn)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn get_welcome(
    conn: &mut PgConnection,
    guild_id: i64,
) -> sqlx::Result<Option<WelcomeConfig>> {
    sqlx::query_as::<_, WelcomeConfig>(
        "SELECT guild_id, channel_id, auto_role_id, title, message, enabled
         FROM guild_welcome_configs WHERE guild_id = $1",
    )
    .bind(guild_id)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn list_all_welcomes(conn: &mut PgConnection) -> sqlx::Result<Vec<WelcomeConfig>> {
    sqlx::query_as::<_, WelcomeConfig>(
        "SELECT guild_id, channel_id, auto_role_id, title, message, enabled
         FROM guild_welcome_configs",
    )
    .fetch_all(&mut *conn)
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn create_giveaway(
    conn: &mut PgConnection,
    guild_id: i64,
    channel_id: i64,
    host_id: i64,
    kind: &str,
    prize: &str,
    cash_amount: Decimal,
    winner_count: i32,
    ends_at: DateTime<Utc>,
) -> sqlx::Result<Giveaway> {
    sqlx::query_as::<_, Giveaway>(
        "INSERT INTO giveaways (guild_id, channel_id, host_id, kind, prize, cash_amount, winner_count, ends_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, guild_id, channel_id, message_id, host_id, kind, prize, cash_amount, winner_count, ends_at, status, winner_ids",
    )
    .bind(guild_id)
    .bind(channel_id)
    .bind(host_id)
    .bind(kind)
    .bind(prize)
    .bind(cash_amount)
    .bind(winner_count)
    .bind(ends_at)
    .fetch_one(&mut *conn)
    .await
}

pub async fn set_giveaway_message_id(
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

pub async fn get_giveaway(conn: &mut PgConnection, id: i64) -> sqlx::Result<Option<Giveaway>> {
    sqlx::query_as::<_, Giveaway>(
        "SELECT id, guild_id, channel_id, message_id, host_id, kind, prize, cash_amount, winner_count, ends_at, status, winner_ids
         FROM giveaways WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn lock_giveaway(conn: &mut PgConnection, id: i64) -> sqlx::Result<Option<Giveaway>> {
    sqlx::query_as::<_, Giveaway>(
        "SELECT id, guild_id, channel_id, message_id, host_id, kind, prize, cash_amount, winner_count, ends_at, status, winner_ids
         FROM giveaways WHERE id = $1 FOR UPDATE",
    )
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

pub async fn mark_giveaway_ended(
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

pub async fn update_giveaway_winners(
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
