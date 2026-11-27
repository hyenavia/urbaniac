use crate::domain::i18n::Lang;
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
        "SELECT discord_user_id, username, avatar_url, level, xp, created_at, lang
         FROM players WHERE discord_user_id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn get_lang(conn: &mut PgConnection, id: i64) -> sqlx::Result<Lang> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT lang FROM players WHERE discord_user_id = $1")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?;
    Ok(raw.as_deref().map(Lang::parse).unwrap_or(Lang::En))
}

pub async fn set_lang(conn: &mut PgConnection, id: i64, lang: Lang) -> sqlx::Result<()> {
    sqlx::query("UPDATE players SET lang = $2, updated_at = now() WHERE discord_user_id = $1")
        .bind(id)
        .bind(lang.as_str())
        .execute(&mut *conn)
        .await?;
    Ok(())
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

/// Mengunci baris pemain (urutan kunci: pemain -> baris lain). false jika pemain tidak ada.
pub async fn lock(conn: &mut PgConnection, id: i64) -> sqlx::Result<bool> {
    Ok(sqlx::query("SELECT 1 FROM players WHERE discord_user_id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .is_some())
}

pub async fn set_progress(conn: &mut PgConnection, id: i64, level: i32, xp: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE players SET level = $2, xp = $3, updated_at = now() WHERE discord_user_id = $1")
        .bind(id)
        .bind(level)
        .bind(xp)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn get_thread_id(conn: &mut PgConnection, id: i64) -> sqlx::Result<Option<i64>> {
    let tid: Option<Option<i64>> =
        sqlx::query_scalar("SELECT thread_id FROM players WHERE discord_user_id = $1")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?;
    Ok(tid.flatten())
}

pub async fn set_thread_id(conn: &mut PgConnection, id: i64, thread_id: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE players SET thread_id = $2, updated_at = now() WHERE discord_user_id = $1")
        .bind(id)
        .bind(thread_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn claim_progression_reward_if_absent(
    conn: &mut PgConnection,
    player_id: i64,
    reward_key: &str,
) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "INSERT INTO player_progression_rewards (player_id, reward_key)
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(player_id)
    .bind(reward_key)
    .execute(&mut *conn)
    .await?;
    Ok(res.rows_affected() == 1)
}
