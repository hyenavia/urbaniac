use crate::domain::city::{inventory_capacity_with_granary, ProjectId};
use crate::domain::resources::{find_item, ItemDef, BASE_SLOT_CAPACITY};
use crate::infrastructure::repositories::city_repo;
use crate::infrastructure::repositories::inventory_repo::{self, ClaimResult};
use sqlx::{PgConnection, PgPool};

pub struct InventoryLine {
    pub item: &'static ItemDef,
    pub quantity: i64,
}

pub struct InventoryView {
    pub lines: Vec<InventoryLine>,
    pub pending: Vec<InventoryLine>,
    pub slots_used: i64,
    pub slot_capacity: i64,
}

/// Kapasitas slot pemain: dasar + bonus Lumbung.
pub async fn slot_capacity_conn(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<i64> {
    let granary_lvl =
        city_repo::facility_level(conn, player_id, ProjectId::Granary.as_str()).await?;
    Ok(inventory_capacity_with_granary(BASE_SLOT_CAPACITY, granary_lvl))
}

pub async fn slot_capacity(pool: &PgPool, player_id: i64) -> anyhow::Result<i64> {
    let mut conn = pool.acquire().await?;
    Ok(slot_capacity_conn(&mut *conn, player_id).await?)
}

fn to_lines(rows: Vec<(String, i64)>) -> Vec<InventoryLine> {
    rows.into_iter()
        .filter_map(|(id, quantity)| find_item(&id).map(|item| InventoryLine { item, quantity }))
        .collect()
}

pub async fn view(pool: &PgPool, player_id: i64) -> anyhow::Result<InventoryView> {
    let mut conn = pool.acquire().await?;
    let lines = to_lines(inventory_repo::list_items(&mut *conn, player_id).await?);
    let pending = to_lines(inventory_repo::list_pending(&mut *conn, player_id).await?);
    let cap = slot_capacity_conn(&mut *conn, player_id).await?;
    Ok(InventoryView {
        slots_used: lines.len() as i64,
        lines,
        pending,
        slot_capacity: cap,
    })
}

pub async fn claim_pending(pool: &PgPool, player_id: i64) -> anyhow::Result<Vec<ClaimResult>> {
    let mut tx = pool.begin().await?;
    let cap = slot_capacity_conn(&mut *tx, player_id).await?;
    let res = inventory_repo::claim_pending(&mut *tx, player_id, cap).await?;
    tx.commit().await?;
    Ok(res)
}
