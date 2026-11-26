use crate::domain::player::Player;
use sqlx::PgConnection;

/// Mengembalikan true jika pemain baru dibuat.
pub async fn insert_if_absent(
    conn: &mut PgConnection,
    id: i64,
    username: &str,
    avatar: Option<&str>,
) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "INSERT INTO players (discord_user_id, username, avatar_url) VALUES ($1, $2, $3)
         ON CONFLICT (discord_user_id) DO NOTHING",
    )
    .bind(id)
    .bind(username)
    .bind(avatar)
    .execute(&mut *conn)
    .await?;
    Ok(res.rows_affected() == 1)
}

pub async fn get(conn: &mut PgConnection, id: i64) -> sqlx::Result<Option<Player>> {
    sqlx::query_as::<_, Player>(
        "SELECT discord_user_id, username, avatar_url, level, xp, created_at
         FROM players WHERE discord_user_id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn update_profile(
    conn: &mut PgConnection,
    id: i64,
    username: &str,
    avatar: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE players SET username = $2, avatar_url = $3, updated_at = now()
         WHERE discord_user_id = $1",
    )
    .bind(id)
    .bind(username)
    .bind(avatar)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
