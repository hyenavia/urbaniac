use crate::domain::resources::{find_item, plan_add, ItemDef, Placement};
use sqlx::PgConnection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimResult {
    pub item_id: String,
    pub claimed: i64,
    pub remaining: i64,
}

/// Mengunci baris pemain agar operasi inventory pemain yang sama berjalan berurutan.
async fn lock_player(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<()> {
    sqlx::query("SELECT 1 FROM players WHERE discord_user_id = $1 FOR UPDATE")
        .bind(player_id)
        .fetch_one(&mut *conn)
        .await?;
    Ok(())
}

async fn place(
    conn: &mut PgConnection,
    player_id: i64,
    item: &ItemDef,
    qty: i64,
    slot_capacity: i64,
) -> sqlx::Result<Placement> {
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT quantity FROM inventory_items WHERE player_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(item.id)
    .fetch_optional(&mut *conn)
    .await?;
    let used: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM inventory_items WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(&mut *conn)
        .await?;

    let p = plan_add(existing, used, slot_capacity, item.stack_limit, qty);
    if p.stored > 0 {
        sqlx::query(
            "INSERT INTO inventory_items (player_id, item_id, quantity) VALUES ($1, $2, $3)
             ON CONFLICT (player_id, item_id)
             DO UPDATE SET quantity = inventory_items.quantity + EXCLUDED.quantity",
        )
        .bind(player_id)
        .bind(item.id)
        .bind(p.stored)
        .execute(&mut *conn)
        .await?;
    }
    Ok(p)
}

/// Menambah item; sisa yang tidak muat masuk Pending Claim.
pub async fn add_item(
    conn: &mut PgConnection,
    player_id: i64,
    item: &ItemDef,
    qty: i64,
    slot_capacity: i64,
) -> sqlx::Result<Placement> {
    if qty <= 0 {
        return Ok(Placement { stored: 0, pending: 0 });
    }
    lock_player(conn, player_id).await?;
    let p = place(conn, player_id, item, qty, slot_capacity).await?;
    if p.pending > 0 {
        sqlx::query(
            "INSERT INTO pending_claims (player_id, item_id, quantity) VALUES ($1, $2, $3)
             ON CONFLICT (player_id, item_id)
             DO UPDATE SET quantity = pending_claims.quantity + EXCLUDED.quantity, updated_at = now()",
        )
        .bind(player_id)
        .bind(item.id)
        .bind(p.pending)
        .execute(&mut *conn)
        .await?;
    }
    Ok(p)
}

/// Mengurangi item. Mengembalikan false jika jumlah tidak cukup (tidak ada perubahan).
pub async fn remove_item(
    conn: &mut PgConnection,
    player_id: i64,
    item_id: &str,
    qty: i64,
) -> sqlx::Result<bool> {
    if qty <= 0 {
        return Ok(true);
    }
    lock_player(conn, player_id).await?;
    let res = sqlx::query(
        "UPDATE inventory_items SET quantity = quantity - $3
         WHERE player_id = $1 AND item_id = $2 AND quantity >= $3",
    )
    .bind(player_id)
    .bind(item_id)
    .bind(qty)
    .execute(&mut *conn)
    .await?;
    if res.rows_affected() == 0 {
        return Ok(false);
    }
    sqlx::query("DELETE FROM inventory_items WHERE player_id = $1 AND item_id = $2 AND quantity = 0")
        .bind(player_id)
        .bind(item_id)
        .execute(&mut *conn)
        .await?;
    Ok(true)
}

pub async fn list_items(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<Vec<(String, i64)>> {
    sqlx::query_as::<_, (String, i64)>(
        "SELECT item_id, quantity FROM inventory_items WHERE player_id = $1 ORDER BY item_id",
    )
    .bind(player_id)
    .fetch_all(&mut *conn)
    .await
}

pub async fn list_pending(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<Vec<(String, i64)>> {
    sqlx::query_as::<_, (String, i64)>(
        "SELECT item_id, quantity FROM pending_claims WHERE player_id = $1 ORDER BY item_id",
    )
    .bind(player_id)
    .fetch_all(&mut *conn)
    .await
}

/// Memindahkan pending ke inventory sebanyak yang muat.
pub async fn claim_pending(
    conn: &mut PgConnection,
    player_id: i64,
    slot_capacity: i64,
) -> sqlx::Result<Vec<ClaimResult>> {
    lock_player(conn, player_id).await?;
    let rows = list_pending(conn, player_id).await?;
    let mut out = Vec::new();
    for (item_id, qty) in rows {
        let Some(item) = find_item(&item_id) else { continue };
        let p = place(conn, player_id, item, qty, slot_capacity).await?;
        if p.pending == 0 {
            sqlx::query("DELETE FROM pending_claims WHERE player_id = $1 AND item_id = $2")
                .bind(player_id)
                .bind(&item_id)
                .execute(&mut *conn)
                .await?;
        } else if p.stored > 0 {
            sqlx::query(
                "UPDATE pending_claims SET quantity = $3, updated_at = now()
                 WHERE player_id = $1 AND item_id = $2",
            )
            .bind(player_id)
            .bind(&item_id)
            .bind(p.pending)
            .execute(&mut *conn)
            .await?;
        }
        out.push(ClaimResult { item_id, claimed: p.stored, remaining: p.pending });
    }
    Ok(out)
}
