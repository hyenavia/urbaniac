//! Antrean job persisten di PostgreSQL. Job bertahan saat bot restart.
use chrono::{DateTime, Utc};
use sqlx::PgConnection;

pub const MAX_ATTEMPTS: i32 = 5;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DueJob {
    pub id: i64,
    pub kind: String,
    pub ref_id: i64,
}

/// Idempoten: (kind, ref_id) yang sama tidak diantrekan dua kali.
pub async fn enqueue(
    conn: &mut PgConnection,
    kind: &str,
    ref_id: i64,
    run_at: DateTime<Utc>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO scheduled_jobs (kind, ref_id, run_at) VALUES ($1, $2, $3)
         ON CONFLICT (kind, ref_id) DO NOTHING",
    )
    .bind(kind)
    .bind(ref_id)
    .bind(run_at)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn due_jobs(conn: &mut PgConnection, limit: i64) -> sqlx::Result<Vec<DueJob>> {
    sqlx::query_as::<_, DueJob>(
        "SELECT id, kind, ref_id FROM scheduled_jobs
         WHERE status = 'pending' AND run_at <= now() ORDER BY run_at, id LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&mut *conn)
    .await
}

/// Mengunci job pending. false jika sudah diproses atau dikunci pihak lain.
pub async fn lock_pending(conn: &mut PgConnection, id: i64) -> sqlx::Result<bool> {
    Ok(sqlx::query("SELECT id FROM scheduled_jobs WHERE id = $1 AND status = 'pending' FOR UPDATE SKIP LOCKED")
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .is_some())
}

pub async fn mark_done(conn: &mut PgConnection, id: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE scheduled_jobs SET status = 'done', finished_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Mencatat kegagalan; coba lagi 30 detik kemudian, gagal permanen setelah MAX_ATTEMPTS.
pub async fn record_failure(conn: &mut PgConnection, id: i64, error: &str) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE scheduled_jobs
         SET attempts = attempts + 1,
             last_error = $2,
             status = CASE WHEN attempts + 1 >= $3 THEN 'failed' ELSE 'pending' END,
             run_at = now() + interval '30 seconds'
         WHERE id = $1",
    )
    .bind(id)
    .bind(error)
    .bind(MAX_ATTEMPTS)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn next_run_at(conn: &mut PgConnection) -> sqlx::Result<Option<DateTime<Utc>>> {
    sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
        "SELECT MIN(run_at) FROM scheduled_jobs WHERE status = 'pending'",
    )
    .fetch_one(&mut *conn)
    .await
}
