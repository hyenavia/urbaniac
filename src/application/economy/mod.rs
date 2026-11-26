use crate::infrastructure::repositories::wallet_repo::{self, ApplyOutcome, WalletError};
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
