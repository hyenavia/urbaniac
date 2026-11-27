use crate::web::error::ApiError;
use crate::web::WebState;
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use serde::Deserialize;
use serenity::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DISCORD_API_BASE: &str = "https://discord.com/api/v10";
const DEFAULT_CACHE_MAX_ENTRIES: usize = 512;
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(300);
const API_RATE_LIMIT_MAX: u32 = 60;
const API_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthUser {
    pub user_id: i64,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[async_trait]
pub trait DiscordAuthProvider: Send + Sync {
    async fn exchange_code(&self, code: &str) -> Result<String, ApiError>;
    async fn verify_token(&self, bearer_token: &str) -> Result<AuthUser, ApiError>;
}

#[derive(Debug, Clone)]
struct CachedEntry {
    user: AuthUser,
    expires_at: Instant,
}

/// Bounded TTL cache mapping Discord OAuth2 bearer token -> verified AuthUser.
pub struct TokenCache {
    entries: Mutex<HashMap<String, CachedEntry>>,
    max_entries: usize,
    ttl: Duration,
}

impl Default for TokenCache {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_MAX_ENTRIES, DEFAULT_CACHE_TTL)
    }
}

impl TokenCache {
    pub fn new(max_entries: usize, ttl: Duration) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            max_entries: max_entries.max(1),
            ttl,
        }
    }

    pub fn get(&self, token: &str) -> Option<AuthUser> {
        self.get_at(token, Instant::now())
    }

    fn get_at(&self, token: &str, now: Instant) -> Option<AuthUser> {
        let mut map = self.entries.lock().expect("token cache mutex poisoned");
        if let Some(entry) = map.get(token) {
            if now < entry.expires_at {
                return Some(entry.user.clone());
            }
        }
        map.remove(token);
        None
    }

    pub fn insert(&self, token: String, user: AuthUser) {
        self.insert_at(token, user, Instant::now());
    }

    fn insert_at(&self, token: String, user: AuthUser, now: Instant) {
        let mut map = self.entries.lock().expect("token cache mutex poisoned");
        let expires_at = now + self.ttl;

        if map.len() >= self.max_entries && !map.contains_key(&token) {
            map.retain(|_, v| now < v.expires_at);
            if map.len() >= self.max_entries {
                if let Some(oldest_key) = map
                    .iter()
                    .min_by_key(|(_, v)| v.expires_at)
                    .map(|(k, _)| k.clone())
                {
                    map.remove(&oldest_key);
                }
            }
        }

        map.insert(token, CachedEntry { user, expires_at });
    }
}

pub struct HttpDiscordAuth {
    http: reqwest::Client,
    client_id: String,
    client_secret: String,
    cache: TokenCache,
}

impl HttpDiscordAuth {
    pub fn new(client_id: String, client_secret: String) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            http,
            client_id,
            client_secret,
            cache: TokenCache::default(),
        })
    }
}

#[derive(Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct DiscordUserResponse {
    id: String,
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
}

#[async_trait]
impl DiscordAuthProvider for HttpDiscordAuth {
    async fn exchange_code(&self, code: &str) -> Result<String, ApiError> {
        let resp = self
            .http
            .post(format!("{DISCORD_API_BASE}/oauth2/token"))
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("grant_type", "authorization_code"),
                ("code", code),
            ])
            .send()
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

        let status = resp.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(ApiError::too_many_requests(
                "discord_rate_limited",
                "Discord OAuth rate limited, please retry shortly",
            ));
        }
        if status == reqwest::StatusCode::BAD_REQUEST || status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ApiError::unauthorized(
                "invalid_code",
                "Invalid or expired Discord authorization code",
            ));
        }
        if !status.is_success() {
            return Err(ApiError::Internal(anyhow::anyhow!(
                "Discord token exchange returned HTTP {}",
                status
            )));
        }

        let body: DiscordTokenResponse = resp
            .json()
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
        if body.access_token.trim().is_empty() {
            return Err(ApiError::unauthorized(
                "invalid_token_response",
                "Discord returned an empty access token",
            ));
        }
        Ok(body.access_token)
    }

    async fn verify_token(&self, bearer_token: &str) -> Result<AuthUser, ApiError> {
        if let Some(cached) = self.cache.get(bearer_token) {
            return Ok(cached);
        }

        let resp = self
            .http
            .get(format!("{DISCORD_API_BASE}/users/@me"))
            .bearer_auth(bearer_token)
            .send()
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(ApiError::unauthorized(
                "invalid_token",
                "Invalid or expired Discord access token",
            ));
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(ApiError::too_many_requests(
                "discord_rate_limited",
                "Discord verification rate limited, please retry shortly",
            ));
        }
        if !status.is_success() {
            return Err(ApiError::Internal(anyhow::anyhow!(
                "Discord /users/@me returned HTTP {}",
                status
            )));
        }

        let raw: DiscordUserResponse = resp
            .json()
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
        let user_id_u64 = raw.id.parse::<u64>().map_err(|_| {
            ApiError::Internal(anyhow::anyhow!("invalid Discord user id: {}", raw.id))
        })?;
        let user_id = i64::try_from(user_id_u64).map_err(|_| {
            ApiError::Internal(anyhow::anyhow!("Discord user id out of i64 range: {}", raw.id))
        })?;
        let avatar_url = raw
            .avatar
            .filter(|h| !h.is_empty())
            .map(|h| format!("https://cdn.discordapp.com/avatars/{user_id_u64}/{h}.png"));
        let username = raw
            .global_name
            .filter(|n| !n.trim().is_empty())
            .unwrap_or(raw.username);

        let user = AuthUser { user_id, username, avatar_url };
        self.cache.insert(bearer_token.to_string(), user.clone());
        Ok(user)
    }
}

fn extract_bearer(parts: &Parts) -> Option<&str> {
    let raw = parts.headers.get(AUTHORIZATION)?.to_str().ok()?.trim();
    let token = raw.strip_prefix("Bearer ").or_else(|| raw.strip_prefix("bearer "))?;
    let token = token.trim();
    if token.is_empty() { None } else { Some(token) }
}

impl FromRequestParts<Arc<WebState>> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<WebState>,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_bearer(parts).ok_or_else(|| {
            ApiError::unauthorized("missing_token", "Authorization Bearer token is required")
        })?;

        let user = state.auth.verify_token(token).await?;
        let rate_key = format!("api:{}", user.user_id);
        if !state
            .rate_limiter
            .check(&rate_key, API_RATE_LIMIT_MAX, API_RATE_LIMIT_WINDOW)
        {
            return Err(ApiError::too_many_requests(
                "rate_limited",
                "Too many requests; slow down",
            ));
        }

        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_user(id: i64, name: &str) -> AuthUser {
        AuthUser {
            user_id: id,
            username: name.to_string(),
            avatar_url: None,
        }
    }

    #[test]
    fn cache_hits_and_expires_after_ttl() {
        let cache = TokenCache::new(4, Duration::from_secs(60));
        let t0 = Instant::now();
        cache.insert_at("tok1".into(), sample_user(10, "alice"), t0);

        assert_eq!(
            cache.get_at("tok1", t0 + Duration::from_secs(30)),
            Some(sample_user(10, "alice"))
        );
        assert_eq!(cache.get_at("tok1", t0 + Duration::from_secs(60)), None);
    }

    #[test]
    fn cache_evicts_oldest_when_full() {
        let cache = TokenCache::new(2, Duration::from_secs(60));
        let t0 = Instant::now();
        cache.insert_at("t1".into(), sample_user(1, "u1"), t0);
        cache.insert_at("t2".into(), sample_user(2, "u2"), t0 + Duration::from_secs(1));
        cache.insert_at("t3".into(), sample_user(3, "u3"), t0 + Duration::from_secs(2));

        assert_eq!(cache.get_at("t1", t0 + Duration::from_secs(3)), None);
        assert_eq!(cache.get_at("t2", t0 + Duration::from_secs(3)), Some(sample_user(2, "u2")));
        assert_eq!(cache.get_at("t3", t0 + Duration::from_secs(3)), Some(sample_user(3, "u3")));
    }
}
