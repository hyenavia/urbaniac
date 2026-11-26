use crate::domain::resources::{find_item, ItemDef, BASE_SLOT_CAPACITY};
use crate::infrastructure::repositories::inventory_repo::{self, ClaimResult};
use sqlx::PgPool;

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

/// Kapasitas slot pemain. Tahap City akan menambah bonus dari Lumbung di sini.
pub fn slot_capacity(_player_id: i64) -> i64 {
    BASE_SLOT_CAPACITY
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
    Ok(InventoryView {
        slots_used: lines.len() as i64,
        lines,
        pending,
        slot_capacity: slot_capacity(player_id),
    })
}

pub async fn claim_pending(pool: &PgPool, player_id: i64) -> anyhow::Result<Vec<ClaimResult>> {
    let mut tx = pool.begin().await?;
    let res = inventory_repo::claim_pending(&mut *tx, player_id, slot_capacity(player_id)).await?;
    tx.commit().await?;
    Ok(res)
}
