use sqlx::PgConnection;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CalendarRow {
    pub day: i32,
    pub season: String,
    pub active_event: Option<String>,
    pub trend_item_id: Option<String>,
}

pub async fn get_calendar(conn: &mut PgConnection) -> sqlx::Result<CalendarRow> {
    sqlx::query_as::<_, CalendarRow>(
        "SELECT day, season, active_event, trend_item_id FROM world_calendar WHERE id = 1",
    )
    .fetch_one(&mut *conn)
    .await
}

pub async fn save_calendar(
    conn: &mut PgConnection,
    day: i32,
    season: &str,
    active_event: Option<&str>,
    trend_item_id: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE world_calendar
         SET day = $1, season = $2, active_event = $3, trend_item_id = $4, updated_at = now()
         WHERE id = 1",
    )
    .bind(day)
    .bind(season)
    .bind(active_event)
    .bind(trend_item_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
