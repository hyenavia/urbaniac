use anyhow::{Context, Result};

pub const MAX_DB_CONNECTIONS: u32 = 5;

#[derive(Debug, Clone)]
pub struct Config {
    pub discord_token: String,
    pub database_url: String,
    pub guild_id: Option<u64>,
    pub db_max_connections: u32,
    /// Skala durasi aktivitas dalam persen (100 = normal). Berguna untuk testing.
    pub activity_duration_pct: i64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let discord_token = std::env::var("DISCORD_TOKEN").context("DISCORD_TOKEN belum diisi")?;
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL belum diisi")?;
        let guild_id = match std::env::var("DISCORD_GUILD_ID") {
            Ok(v) if !v.trim().is_empty() => {
                Some(v.trim().parse::<u64>().context("DISCORD_GUILD_ID harus angka")?)
            }
            _ => None,
        };
        let db_max_connections = std::env::var("DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(MAX_DB_CONNECTIONS)
            .clamp(1, MAX_DB_CONNECTIONS);
        let activity_duration_pct = std::env::var("ACTIVITY_DURATION_PCT")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(100)
            .clamp(1, 1000);
        Ok(Self { discord_token, database_url, guild_id, db_max_connections, activity_duration_pct })
    }
}
