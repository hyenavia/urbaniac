use chrono::{DateTime, Utc};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Player {
    pub discord_user_id: i64,
    pub username: String,
    pub avatar_url: Option<String>,
    pub level: i32,
    /// XP di dalam level saat ini.
    pub xp: i64,
    pub created_at: DateTime<Utc>,
}

/// XP yang dibutuhkan untuk naik dari `level` ke `level + 1`. Nilai awal sementara.
pub fn xp_to_next(level: i32) -> i64 {
    100 * level.max(1) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xp_curve_is_linear() {
        assert_eq!(xp_to_next(1), 100);
        assert_eq!(xp_to_next(5), 500);
        assert_eq!(xp_to_next(0), 100);
    }
}
