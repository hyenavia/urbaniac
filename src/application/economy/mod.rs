use crate::application::market;
use crate::domain::resources::{find_item, ItemDef};
use crate::infrastructure::repositories::wallet_repo::{self, ApplyOutcome, WalletError};
use crate::infrastructure::repositories::{inventory_repo, market_repo, player_repo};
use rust_decimal::Decimal;
use sqlx::PgPool;

/// Kredit/debit Bac dengan ledger dan idempotency key. Atomik.
pub async fn apply(
    pool: &PgPool,
    player_id: i64,
    delta: Decimal,
    reason: &str,
    idempotency_key: &str,
) -> Result<ApplyOutcome, WalletError> {
    let mut tx = pool.begin().await?;
    let out = wallet_repo::apply(&mut *tx, player_id, delta, reason, idempotency_key).await?;
    tx.commit().await?;
    Ok(out)
}

pub async fn balance(pool: &PgPool, player_id: i64) -> anyhow::Result<Decimal> {
    let mut conn = pool.acquire().await?;
    Ok(wallet_repo::balance(&mut *conn, player_id).await?.unwrap_or(Decimal::ZERO))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SellQty {
    Exact(i64),
    All,
}

#[derive(Debug)]
pub enum SaleOutcome {
    Sold { item: &'static ItemDef, qty: i64, total: Decimal, balance: Decimal },
    /// Idempotency key sudah dipakai: penjualan ini sudah pernah diproses.
    Duplicate,
}

#[derive(Debug, thiserror::Error)]
pub enum SellError {
    #[error("item tidak dikenal")]
    UnknownItem,
    #[error("jumlah tidak valid")]
    InvalidQty,
    #[error("jumlah item tidak cukup (kamu punya {have})")]
    NotEnough { have: i64 },
    #[error(transparent)]
    Wallet(#[from] WalletError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

/// Harga jual ke NPC saat ini (memperhitungkan pasar dinamis + bonus Pasar Desa +5%).
pub async fn sell_unit_price_conn(
    conn: &mut sqlx::PgConnection,
    player_id: i64,
    item: &ItemDef,
) -> sqlx::Result<Decimal> {
    let base = market::current_npc_price_conn(conn, item).await?;
    let bonus = crate::application::city::market_hall_bonus_pct_conn(conn, player_id).await?;
    if bonus > 0 {
        Ok((base * Decimal::from(100 + bonus) / Decimal::from(100)).round_dp(2))
    } else {
        Ok(base)
    }
}

pub async fn sell_unit_price(pool: &PgPool, player_id: i64, item: &ItemDef) -> anyhow::Result<Decimal> {
    let mut conn = pool.acquire().await?;
    Ok(sell_unit_price_conn(&mut *conn, player_id, item).await?)
}

/// Jual item ke NPC. Pengurangan item dan kredit Bac terjadi dalam satu transaksi.
/// `idempotency_key` harus unik per permintaan jual (mis. ID interaksi Discord).
pub async fn sell_to_npc(
    pool: &PgPool,
    player_id: i64,
    item_id: &str,
    qty: SellQty,
    idempotency_key: &str,
) -> Result<SaleOutcome, SellError> {
    let item = find_item(item_id).ok_or(SellError::UnknownItem)?;
    let mut tx = pool.begin().await?;
    if !player_repo::lock(&mut *tx, player_id).await? {
        return Err(SellError::NotEnough { have: 0 });
    }
    let have = inventory_repo::quantity_of(&mut *tx, player_id, item.id).await?;
    let qty = match qty {
        SellQty::Exact(n) if n <= 0 => return Err(SellError::InvalidQty),
        SellQty::Exact(n) => n,
        SellQty::All => have,
    };
    if qty <= 0 || have < qty {
        return Err(SellError::NotEnough { have });
    }
    inventory_repo::remove_item(&mut *tx, player_id, item.id, qty).await?;
    let unit_price = sell_unit_price_conn(&mut *tx, player_id, item).await?;
    market_repo::record_sale_volume(&mut *tx, item.id, qty).await?;
    let total = unit_price * Decimal::from(qty);
    let reason = format!("npc_sale:{}x{}", item.id, qty);
    match wallet_repo::apply(&mut *tx, player_id, total, &reason, idempotency_key).await? {
        ApplyOutcome::Duplicate => Ok(SaleOutcome::Duplicate), // tx di-drop => rollback
        ApplyOutcome::Applied { balance } => {
            tx.commit().await?;
            Ok(SaleOutcome::Sold { item, qty, total, balance })
        }
    }
}
