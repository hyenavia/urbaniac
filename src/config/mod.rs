use anyhow::{bail, Context, Result};
use std::net::SocketAddr;

pub const MAX_DB_CONNECTIONS: u32 = 5;

#[derive(Debug, Clone)]
pub struct ActivityConfig {
    pub enabled: bool,
    pub bind_addr: String,
    pub static_dir: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub discord_token: String,
    pub database_url: String,
    pub guild_id: Option<u64>,
    pub db_max_connections: u32,
    /// Skala durasi aktivitas dalam persen (100 = normal). Berguna untuk testing.
    pub activity_duration_pct: i64,
    pub activity: ActivityConfig,
}

fn non_empty<F>(get: &F, key: &str) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    get(key)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    pub fn from_lookup<F>(get: F) -> Result<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let discord_token =
            non_empty(&get, "DISCORD_TOKEN").context("DISCORD_TOKEN belum diisi")?;
        let database_url = non_empty(&get, "DATABASE_URL").context("DATABASE_URL belum diisi")?;
        let guild_id = match non_empty(&get, "DISCORD_GUILD_ID") {
            Some(v) => Some(v.parse::<u64>().context("DISCORD_GUILD_ID harus angka")?),
            None => None,
        };
        let db_max_connections = non_empty(&get, "DB_MAX_CONNECTIONS")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(MAX_DB_CONNECTIONS)
            .clamp(1, MAX_DB_CONNECTIONS);
        let activity_duration_pct = non_empty(&get, "ACTIVITY_DURATION_PCT")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(100)
            .clamp(1, 1000);

        let enabled = non_empty(&get, "ACTIVITY_ENABLED")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1"))
            .unwrap_or(false);
        let bind_addr =
            non_empty(&get, "ACTIVITY_BIND_ADDR").unwrap_or_else(|| "127.0.0.1:8080".to_string());
        let static_dir =
            non_empty(&get, "ACTIVITY_STATIC_DIR").unwrap_or_else(|| "./activity-dist".to_string());
        let client_id = non_empty(&get, "DISCORD_CLIENT_ID");
        let client_secret = non_empty(&get, "DISCORD_CLIENT_SECRET");

        if enabled {
            bind_addr
                .parse::<SocketAddr>()
                .context("ACTIVITY_BIND_ADDR harus format host:port yang valid")?;
            if client_id.is_none() || client_secret.is_none() {
                bail!("ACTIVITY_ENABLED=true membutuhkan DISCORD_CLIENT_ID dan DISCORD_CLIENT_SECRET");
            }
        }

        Ok(Self {
            discord_token,
            database_url,
            guild_id,
            db_max_connections,
            activity_duration_pct,
            activity: ActivityConfig {
                enabled,
                bind_addr,
                static_dir,
                client_id,
                client_secret,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn lookup_from<'a>(
        pairs: &'a [(&'a str, &'a str)],
    ) -> impl Fn(&str) -> Option<String> + 'a {
        let map: HashMap<&str, &str> = pairs.iter().copied().collect();
        move |k| map.get(k).map(|v| (*v).to_string())
    }

    #[test]
    fn activity_defaults_to_disabled_without_oauth_credentials() {
        let cfg = Config::from_lookup(lookup_from(&[
            ("DISCORD_TOKEN", "bot-token"),
            ("DATABASE_URL", "postgres://localhost/urbaniac"),
        ]))
        .unwrap();
        assert!(!cfg.activity.enabled);
        assert_eq!(cfg.activity.bind_addr, "127.0.0.1:8080");
        assert_eq!(cfg.activity.static_dir, "./activity-dist");
        assert!(cfg.activity.client_id.is_none());
        assert!(cfg.activity.client_secret.is_none());
    }

    #[test]
    fn activity_enabled_requires_oauth_credentials_and_valid_bind_addr() {
        let err = Config::from_lookup(lookup_from(&[
            ("DISCORD_TOKEN", "bot-token"),
            ("DATABASE_URL", "postgres://localhost/urbaniac"),
            ("ACTIVITY_ENABLED", "true"),
        ]))
        .unwrap_err();
        assert!(err.to_string().contains("DISCORD_CLIENT_ID"));

        let bad_addr = Config::from_lookup(lookup_from(&[
            ("DISCORD_TOKEN", "bot-token"),
            ("DATABASE_URL", "postgres://localhost/urbaniac"),
            ("ACTIVITY_ENABLED", "1"),
            ("ACTIVITY_BIND_ADDR", "not-an-addr"),
            ("DISCORD_CLIENT_ID", "123"),
            ("DISCORD_CLIENT_SECRET", "sec"),
        ]))
        .unwrap_err();
        assert!(bad_addr.to_string().contains("ACTIVITY_BIND_ADDR"));
    }

    #[test]
    fn activity_enabled_succeeds_with_complete_config() {
        let cfg = Config::from_lookup(lookup_from(&[
            ("DISCORD_TOKEN", "bot-token"),
            ("DATABASE_URL", "postgres://localhost/urbaniac"),
            ("ACTIVITY_ENABLED", "true"),
            ("ACTIVITY_BIND_ADDR", "0.0.0.0:9090"),
            ("ACTIVITY_STATIC_DIR", "./dist"),
            ("DISCORD_CLIENT_ID", "123456"),
            ("DISCORD_CLIENT_SECRET", "secret"),
        ]))
        .unwrap();
        assert!(cfg.activity.enabled);
        assert_eq!(cfg.activity.bind_addr, "0.0.0.0:9090");
        assert_eq!(cfg.activity.static_dir, "./dist");
        assert_eq!(cfg.activity.client_id.as_deref(), Some("123456"));
        assert_eq!(cfg.activity.client_secret.as_deref(), Some("secret"));
    }
}
