use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgConnection;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MarketPriceRow {
    pub item_id: String,
    pub multiplier: Decimal,
    pub recent_sales: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrokerListingRow {
    pub id: i64,
    pub player_id: i64,
    pub item_id: String,
    pub quantity: i64,
    pub target_unit_price: Decimal,
    pub listing_fee: Decimal,
    pub status: String,
    pub sold_unit_price: Option<Decimal>,
    pub commission: Option<Decimal>,
    pub expires_at: DateTime<Utc>,
}

pub async fn list_prices(conn: &mut PgConnection) -> sqlx::Result<Vec<MarketPriceRow>> {
    sqlx::query_as::<_, MarketPriceRow>(
        "SELECT item_id, multiplier, recent_sales, updated_at
         FROM npc_market_prices
         ORDER BY item_id",
    )
    .fetch_all(&mut *conn)
    .await
}

pub async fn get_multiplier(conn: &mut PgConnection, item_id: &str) -> sqlx::Result<Decimal> {
    let m = sqlx::query_scalar::<_, Decimal>(
        "SELECT multiplier FROM npc_market_prices WHERE item_id = $1",
    )
    .bind(item_id)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(m.unwrap_or(Decimal::ONE))
}

pub async fn record_sale_volume(
    conn: &mut PgConnection,
    item_id: &str,
    qty: i64,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE npc_market_prices
         SET recent_sales = recent_sales + $2, updated_at = now()
         WHERE item_id = $1",
    )
    .bind(item_id)
    .bind(qty)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn update_price(
    conn: &mut PgConnection,
    item_id: &str,
    multiplier: Decimal,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE npc_market_prices
         SET multiplier = $2, recent_sales = 0, updated_at = now()
         WHERE item_id = $1",
    )
    .bind(item_id)
    .bind(multiplier)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn count_active_listings(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM broker_listings WHERE player_id = $1 AND status = 'active'",
    )
    .bind(player_id)
    .fetch_one(&mut *conn)
    .await
}

pub async fn list_player_active(
    conn: &mut PgConnection,
    player_id: i64,
) -> sqlx::Result<Vec<BrokerListingRow>> {
    sqlx::query_as::<_, BrokerListingRow>(
        "SELECT id, player_id, item_id, quantity, target_unit_price, listing_fee,
                status, sold_unit_price, commission, expires_at
         FROM broker_listings
         WHERE player_id = $1 AND status = 'active'
         ORDER BY id DESC",
    )
    .bind(player_id)
    .fetch_all(&mut *conn)
    .await
}

pub async fn list_all_active(conn: &mut PgConnection) -> sqlx::Result<Vec<BrokerListingRow>> {
    sqlx::query_as::<_, BrokerListingRow>(
        "SELECT id, player_id, item_id, quantity, target_unit_price, listing_fee,
                status, sold_unit_price, commission, expires_at
         FROM broker_listings
         WHERE status = 'active' AND expires_at > now()
         ORDER BY id",
    )
    .fetch_all(&mut *conn)
    .await
}

pub async fn insert_listing(
    conn: &mut PgConnection,
    player_id: i64,
    item_id: &str,
    quantity: i64,
    target_unit_price: Decimal,
    listing_fee: Decimal,
    duration_secs: i64,
) -> sqlx::Result<(i64, DateTime<Utc>)> {
    sqlx::query_as::<_, (i64, DateTime<Utc>)>(
        "INSERT INTO broker_listings (player_id, item_id, quantity, target_unit_price, listing_fee, status, expires_at)
         VALUES ($1, $2, $3, $4, $5, 'active', now() + make_interval(secs => $6::double precision))
         RETURNING id, expires_at",
    )
    .bind(player_id)
    .bind(item_id)
    .bind(quantity)
    .bind(target_unit_price)
    .bind(listing_fee)
    .bind(duration_secs as f64)
    .fetch_one(&mut *conn)
    .await
}

pub async fn lock_listing(
    conn: &mut PgConnection,
    id: i64,
) -> sqlx::Result<Option<BrokerListingRow>> {
    sqlx::query_as::<_, BrokerListingRow>(
        "SELECT id, player_id, item_id, quantity, target_unit_price, listing_fee,
                status, sold_unit_price, commission, expires_at
         FROM broker_listings
         WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn mark_sold(
    conn: &mut PgConnection,
    id: i64,
    sold_unit_price: Decimal,
    commission: Decimal,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE broker_listings
         SET status = 'sold', sold_unit_price = $2, commission = $3, closed_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(sold_unit_price)
    .bind(commission)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn mark_closed(conn: &mut PgConnection, id: i64, status: &str) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE broker_listings
         SET status = $2, closed_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn has_pending_market_tick(conn: &mut PgConnection) -> sqlx::Result<bool> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT id FROM scheduled_jobs WHERE kind = 'market_tick' AND status = 'pending' LIMIT 1",
    )
    .fetch_optional(&mut *conn)
    .await?
    .is_some())
}

pub async fn next_market_tick_ref(conn: &mut PgConnection) -> sqlx::Result<i64> {
    let max_ref: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(ref_id) FROM scheduled_jobs WHERE kind = 'market_tick'",
    )
    .fetch_one(&mut *conn)
    .await?;
    Ok(max_ref.unwrap_or(0) + 1)
}
