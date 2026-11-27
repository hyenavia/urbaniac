use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgConnection;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CraftingJobRow {
    pub id: i64,
    pub player_id: i64,
    pub recipe_id: String,
    pub quantity: i64,
    pub status: String,
    pub auto_sell: bool,
    pub ends_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ShopRepRow {
    pub player_id: i64,
    pub level: i32,
    pub xp: i64,
    pub completed_orders: i64,
    pub auto_craft: bool,
    pub auto_sell: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NpcOrderRow {
    pub id: i64,
    pub player_id: i64,
    pub item_id: String,
    pub quantity: i64,
    pub reward_bac: Decimal,
    pub rep_xp: i64,
    pub status: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn count_running_crafts(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM crafting_jobs WHERE player_id = $1 AND status = 'running'",
    )
    .bind(player_id)
    .fetch_one(&mut *conn)
    .await
}

pub async fn list_running_crafts(
    conn: &mut PgConnection,
    player_id: i64,
) -> sqlx::Result<Vec<CraftingJobRow>> {
    sqlx::query_as::<_, CraftingJobRow>(
        "SELECT id, player_id, recipe_id, quantity, status, auto_sell, ends_at
         FROM crafting_jobs
         WHERE player_id = $1 AND status = 'running'
         ORDER BY ends_at, id",
    )
    .bind(player_id)
    .fetch_all(&mut *conn)
    .await
}

pub async fn insert_craft(
    conn: &mut PgConnection,
    player_id: i64,
    recipe_id: &str,
    quantity: i64,
    auto_sell: bool,
    duration_secs: i64,
) -> sqlx::Result<(i64, DateTime<Utc>)> {
    sqlx::query_as::<_, (i64, DateTime<Utc>)>(
        "INSERT INTO crafting_jobs (player_id, recipe_id, quantity, status, auto_sell, ends_at)
         VALUES ($1, $2, $3, 'running', $4, now() + make_interval(secs => $5::double precision))
         RETURNING id, ends_at",
    )
    .bind(player_id)
    .bind(recipe_id)
    .bind(quantity)
    .bind(auto_sell)
    .bind(duration_secs as f64)
    .fetch_one(&mut *conn)
    .await
}

pub async fn lock_craft(conn: &mut PgConnection, id: i64) -> sqlx::Result<Option<CraftingJobRow>> {
    sqlx::query_as::<_, CraftingJobRow>(
        "SELECT id, player_id, recipe_id, quantity, status, auto_sell, ends_at
         FROM crafting_jobs WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn mark_craft_completed(conn: &mut PgConnection, id: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE crafting_jobs SET status = 'completed', completed_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn ensure_shop_rep(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<ShopRepRow> {
    sqlx::query(
        "INSERT INTO shop_reputation (player_id) VALUES ($1) ON CONFLICT DO NOTHING",
    )
    .bind(player_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query_as::<_, ShopRepRow>(
        "SELECT player_id, level, xp, completed_orders, auto_craft, auto_sell
         FROM shop_reputation WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_one(&mut *conn)
    .await
}

pub async fn update_shop_rep(
    conn: &mut PgConnection,
    player_id: i64,
    level: i32,
    xp: i64,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE shop_reputation
         SET level = $2, xp = $3, completed_orders = completed_orders + 1, updated_at = now()
         WHERE player_id = $1",
    )
    .bind(player_id)
    .bind(level)
    .bind(xp)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn toggle_auto_sell(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<bool> {
    sqlx::query_scalar::<_, bool>(
        "UPDATE shop_reputation SET auto_sell = NOT auto_sell, updated_at = now()
         WHERE player_id = $1 RETURNING auto_sell",
    )
    .bind(player_id)
    .fetch_one(&mut *conn)
    .await
}

pub async fn list_active_orders(
    conn: &mut PgConnection,
    player_id: i64,
) -> sqlx::Result<Vec<NpcOrderRow>> {
    sqlx::query_as::<_, NpcOrderRow>(
        "SELECT id, player_id, item_id, quantity, reward_bac, rep_xp, status, expires_at
         FROM npc_orders
         WHERE player_id = $1 AND status = 'active'
         ORDER BY id",
    )
    .bind(player_id)
    .fetch_all(&mut *conn)
    .await
}

pub async fn insert_order(
    conn: &mut PgConnection,
    player_id: i64,
    item_id: &str,
    quantity: i64,
    reward_bac: Decimal,
    rep_xp: i64,
) -> sqlx::Result<NpcOrderRow> {
    sqlx::query_as::<_, NpcOrderRow>(
        "INSERT INTO npc_orders (player_id, item_id, quantity, reward_bac, rep_xp, status, expires_at)
         VALUES ($1, $2, $3, $4, $5, 'active', now() + interval '24 hours')
         RETURNING id, player_id, item_id, quantity, reward_bac, rep_xp, status, expires_at",
    )
    .bind(player_id)
    .bind(item_id)
    .bind(quantity)
    .bind(reward_bac)
    .bind(rep_xp)
    .fetch_one(&mut *conn)
    .await
}

pub async fn lock_order(conn: &mut PgConnection, id: i64) -> sqlx::Result<Option<NpcOrderRow>> {
    sqlx::query_as::<_, NpcOrderRow>(
        "SELECT id, player_id, item_id, quantity, reward_bac, rep_xp, status, expires_at
         FROM npc_orders WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn mark_order_fulfilled(conn: &mut PgConnection, id: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE npc_orders SET status = 'fulfilled', fulfilled_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
