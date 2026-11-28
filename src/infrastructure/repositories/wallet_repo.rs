use rust_decimal::Decimal;
use sqlx::PgConnection;

#[derive(Debug, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied { balance: Decimal },
    /// Idempotency key sudah pernah dipakai; tidak ada perubahan.
    Duplicate,
}

#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("Insufficient Bac balance")]
    InsufficientFunds,
    #[error("Wallet not found")]
    NotFound,
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

pub async fn create(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO wallets (player_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(player_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn balance(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<Option<Decimal>> {
    sqlx::query_scalar::<_, Decimal>("SELECT balance FROM wallets WHERE player_id = $1")
        .bind(player_id)
        .fetch_optional(&mut *conn)
        .await
}

/// Satu-satunya jalan mengubah saldo. Wajib dipanggil di dalam transaksi.
/// `delta` positif = kredit, negatif = debit. Saldo tidak pernah negatif.
pub async fn apply(
    conn: &mut PgConnection,
    player_id: i64,
    delta: Decimal,
    reason: &str,
    idempotency_key: &str,
) -> Result<ApplyOutcome, WalletError> {
    let delta = delta.round_dp(2);

    // Kunci baris wallet: menserialisasi operasi pada pemain yang sama.
    let current: Option<Decimal> =
        sqlx::query_scalar("SELECT balance FROM wallets WHERE player_id = $1 FOR UPDATE")
            .bind(player_id)
            .fetch_optional(&mut *conn)
            .await?;
    let current = current.ok_or(WalletError::NotFound)?;

    let exists: Option<i64> =
        sqlx::query_scalar("SELECT id FROM ledger_entries WHERE idempotency_key = $1")
            .bind(idempotency_key)
            .fetch_optional(&mut *conn)
            .await?;
    if exists.is_some() {
        return Ok(ApplyOutcome::Duplicate);
    }

    let new_balance = current + delta;
    if new_balance < Decimal::ZERO {
        return Err(WalletError::InsufficientFunds);
    }

    sqlx::query("UPDATE wallets SET balance = $2, updated_at = now() WHERE player_id = $1")
        .bind(player_id)
        .bind(new_balance)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO ledger_entries (player_id, amount, balance_after, reason, idempotency_key)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(player_id)
    .bind(delta)
    .bind(new_balance)
    .bind(reason)
    .bind(idempotency_key)
    .execute(&mut *conn)
    .await?;

    Ok(ApplyOutcome::Applied { balance: new_balance })
}
