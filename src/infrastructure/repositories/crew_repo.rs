use sqlx::PgConnection;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CrewRow {
    pub id: i64,
    pub player_id: i64,
    pub name: String,
    pub rarity: String,
    pub specialty: String,
    pub level: i32,
    pub assigned_activity: Option<String>,
    pub auto_enabled: bool,
}

pub async fn count_crews(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM player_crews WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(&mut *conn)
        .await
}

pub async fn count_assigned_crews(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM player_crews WHERE player_id = $1 AND assigned_activity IS NOT NULL",
    )
    .bind(player_id)
    .fetch_one(&mut *conn)
    .await
}

pub async fn list_crews(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<Vec<CrewRow>> {
    sqlx::query_as::<_, CrewRow>(
        "SELECT id, player_id, name, rarity, specialty, level, assigned_activity, auto_enabled
         FROM player_crews WHERE player_id = $1 ORDER BY id",
    )
    .bind(player_id)
    .fetch_all(&mut *conn)
    .await
}

pub async fn get_assigned_for_activity(
    conn: &mut PgConnection,
    player_id: i64,
    activity: &str,
) -> sqlx::Result<Option<CrewRow>> {
    sqlx::query_as::<_, CrewRow>(
        "SELECT id, player_id, name, rarity, specialty, level, assigned_activity, auto_enabled
         FROM player_crews
         WHERE player_id = $1 AND assigned_activity = $2
         LIMIT 1",
    )
    .bind(player_id)
    .bind(activity)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn insert_crew(
    conn: &mut PgConnection,
    player_id: i64,
    name: &str,
    rarity: &str,
    specialty: &str,
) -> sqlx::Result<CrewRow> {
    sqlx::query_as::<_, CrewRow>(
        "INSERT INTO player_crews (player_id, name, rarity, specialty)
         VALUES ($1, $2, $3, $4)
         RETURNING id, player_id, name, rarity, specialty, level, assigned_activity, auto_enabled",
    )
    .bind(player_id)
    .bind(name)
    .bind(rarity)
    .bind(specialty)
    .fetch_one(&mut *conn)
    .await
}

pub async fn lock_crew(conn: &mut PgConnection, id: i64) -> sqlx::Result<Option<CrewRow>> {
    sqlx::query_as::<_, CrewRow>(
        "SELECT id, player_id, name, rarity, specialty, level, assigned_activity, auto_enabled
         FROM player_crews WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn set_level(conn: &mut PgConnection, id: i64, level: i32) -> sqlx::Result<()> {
    sqlx::query("UPDATE player_crews SET level = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(level)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn clear_activity_assignment(
    conn: &mut PgConnection,
    player_id: i64,
    activity: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE player_crews
         SET assigned_activity = NULL, auto_enabled = false, updated_at = now()
         WHERE player_id = $1 AND assigned_activity = $2",
    )
    .bind(player_id)
    .bind(activity)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn set_assignment(
    conn: &mut PgConnection,
    id: i64,
    activity: Option<&str>,
    auto_enabled: bool,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE player_crews
         SET assigned_activity = $2, auto_enabled = $3, updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(activity)
    .bind(auto_enabled)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
