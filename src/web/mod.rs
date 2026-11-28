pub mod auth;
pub mod dto;
pub mod error;
pub mod events;
pub mod rate_limit;
pub mod routes;

use crate::application::voice_boost::VoiceBoostTracker;
use crate::config::ActivityConfig;
use crate::web::auth::{DiscordAuthProvider, HttpDiscordAuth};
use crate::web::events::EventHub;
use crate::web::rate_limit::RateLimiter;
use anyhow::Context;
use axum::http::header::{CONTENT_SECURITY_POLICY, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS};
use axum::http::HeaderValue;
use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Notify;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

const MAX_BODY_BYTES: usize = 64 * 1024;
const CSP_POLICY: &str = "default-src 'self'; script-src 'self'; worker-src 'self' blob:; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob: https://cdn.discordapp.com; connect-src 'self' https://discord.com https://*.discordsays.com; frame-ancestors https://discord.com https://*.discord.com https://*.discordsays.com";

pub struct WebState {
    pub pool: PgPool,
    pub notify: Arc<Notify>,
    pub activity_duration_pct: i64,
    pub auth: Arc<dyn DiscordAuthProvider>,
    pub rate_limiter: Arc<RateLimiter>,
    pub events: Arc<EventHub>,
    pub client_id: String,
    pub voice_boost: Arc<VoiceBoostTracker>,
}

pub fn build_router(state: Arc<WebState>, static_dir: Option<&Path>) -> Router {
    let api = Router::new()
        .route("/health", get(routes::health))
        .route("/config", get(routes::app_config))
        .route("/events", get(routes::events_stream))
        .route("/token", post(routes::token_exchange))
        .route("/player/register", post(routes::register_player))
        .route("/player/lang", post(routes::set_player_lang))
        .route("/me", get(routes::me))
        .route("/city", get(routes::get_city))
        .route("/city/projects/quote", get(routes::city_projects_quote))
        .route("/city/projects/start", post(routes::city_projects_start))
        .route("/city/treasury/deposit", post(routes::city_treasury_deposit))
        .route("/city/treasury/withdraw", post(routes::city_treasury_withdraw))
        .route("/city/tax", post(routes::city_set_tax))
        .route("/inventory", get(routes::get_inventory))
        .route("/inventory/claim", post(routes::inventory_claim))
        .route("/market", get(routes::get_market))
        .route("/market/sell", post(routes::market_sell))
        .route("/market/listings", post(routes::create_market_listing))
        .route("/market/listings/cancel", post(routes::cancel_market_listing))
        .route("/crafting", get(routes::get_crafting))
        .route("/crafting/start", post(routes::start_crafting))
        .route("/crafting/auto-sell", post(routes::toggle_crafting_auto_sell))
        .route("/crafting/orders/fulfill", post(routes::fulfill_crafting_order))
        .route("/crafting/orders/refresh", post(routes::refresh_crafting_orders))
        .route("/crew", get(routes::get_crew))
        .route("/crew/recruit", post(routes::recruit_crew))
        .route("/crew/upgrade", post(routes::upgrade_crew))
        .route("/crew/assign", post(routes::assign_crew))
        .route("/activities", get(routes::get_activities))
        .route("/activities/start", post(routes::start_activity))
        .route("/activities/collect", post(routes::collect_activities))
        .route("/{*rest}", get(routes::api_not_found).post(routes::api_not_found))
        .fallback(routes::api_not_found);

    let mut app = Router::new()
        .nest("/api", api)
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        .layer(SetResponseHeaderLayer::overriding(
            X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CSP_POLICY),
        ));

    if let Some(dir) = static_dir {
        let index = dir.join("index.html");
        let serve = ServeDir::new(dir).not_found_service(ServeFile::new(index));
        app = app.fallback_service(serve);
    }

    app
}

pub async fn run(
    cfg: ActivityConfig,
    pool: PgPool,
    notify: Arc<Notify>,
    activity_duration_pct: i64,
    events: Arc<EventHub>,
    voice_boost: Arc<VoiceBoostTracker>,
) -> anyhow::Result<()> {
    let client_id = cfg
        .client_id
        .ok_or_else(|| anyhow::anyhow!("DISCORD_CLIENT_ID belum diisi"))?;
    let client_secret = cfg
        .client_secret
        .ok_or_else(|| anyhow::anyhow!("DISCORD_CLIENT_SECRET belum diisi"))?;

    let auth = Arc::new(HttpDiscordAuth::new(client_id.clone(), client_secret)?);
    let state = Arc::new(WebState {
        pool,
        notify,
        activity_duration_pct,
        auth,
        rate_limiter: Arc::new(RateLimiter::default()),
        events,
        client_id,
        voice_boost,
    });

    let static_path = Path::new(&cfg.static_dir);
    if !static_path.exists() {
        tracing::info!(
            static_dir = %cfg.static_dir,
            "ACTIVITY_STATIC_DIR belum tersedia saat startup; static serving tetap aktif menunggu deployment frontend"
        );
    }

    let router = build_router(state, Some(static_path));
    let listener = match tokio::net::TcpListener::bind(&cfg.bind_addr).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(
                bind_addr = %cfg.bind_addr,
                error = %e,
                "gagal bind port web Activity API"
            );
            return Err(anyhow::Error::new(e))
                .context(format!("gagal bind web Activity API pada {}", cfg.bind_addr));
        }
    };
    tracing::info!(bind_addr = %cfg.bind_addr, "web Activity API berjalan");
    axum::serve(listener, router)
        .await
        .context("server web Activity API berhenti karena error")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web::auth::AuthUser;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serenity::async_trait;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    struct FakeAuth;

    #[async_trait]
    impl DiscordAuthProvider for FakeAuth {
        async fn exchange_code(&self, code: &str) -> Result<String, crate::web::error::ApiError> {
            if code == "valid-code" {
                Ok("valid-token".to_string())
            } else {
                Err(crate::web::error::ApiError::unauthorized(
                    "invalid_code",
                    "Invalid code",
                ))
            }
        }

        async fn verify_token(
            &self,
            bearer_token: &str,
        ) -> Result<AuthUser, crate::web::error::ApiError> {
            if bearer_token == "valid-token" {
                Ok(AuthUser {
                    user_id: 1001,
                    username: "alice".to_string(),
                    avatar_url: None,
                })
            } else {
                Err(crate::web::error::ApiError::unauthorized(
                    "invalid_token",
                    "Invalid token",
                ))
            }
        }
    }

    fn test_router() -> Router {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost:5432/unused")
            .unwrap();
        let state = Arc::new(WebState {
            pool,
            notify: Arc::new(Notify::new()),
            activity_duration_pct: 100,
            auth: Arc::new(FakeAuth),
            rate_limiter: Arc::new(RateLimiter::default()),
            events: Arc::new(EventHub::default()),
            client_id: "test-client-id".to_string(),
            voice_boost: Arc::new(VoiceBoostTracker::disabled()),
        });
        build_router(state, None)
    }

    async fn json_body(resp: axum::response::Response) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_and_security_headers() {
        let app = test_router();
        let resp = app
            .clone()
            .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get(X_CONTENT_TYPE_OPTIONS).unwrap(), "nosniff");
        let csp = resp
            .headers()
            .get(CONTENT_SECURITY_POLICY)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(csp.contains("script-src 'self'"));
        assert!(csp.contains("worker-src 'self' blob:"));
        assert!(!csp.contains("unsafe-eval"));
        assert_eq!(json_body(resp).await["status"], "ok");

        let cfg_resp = app
            .oneshot(Request::builder().uri("/api/config").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(cfg_resp.status(), StatusCode::OK);
        assert_eq!(json_body(cfg_resp).await["discord_client_id"], "test-client-id");
    }

    #[tokio::test]
    async fn protected_routes_reject_missing_and_invalid_tokens() {
        let app = test_router();
        for path in [
            "/api/me",
            "/api/city",
            "/api/inventory",
            "/api/activities",
            "/api/events",
            "/api/market",
            "/api/crafting",
            "/api/crew",
        ] {
            let missing = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(missing.status(), StatusCode::UNAUTHORIZED, "{path} without token");

            let invalid = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(path)
                        .header("authorization", "Bearer wrong-token")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED, "{path} wrong token");
        }
    }

    #[tokio::test]
    async fn token_exchange_and_api_404_json() {
        let app = test_router();

        let ok = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/token")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"code":"valid-code"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK);
        assert_eq!(json_body(ok).await["access_token"], "valid-token");

        let empty = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/token")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"code":"  "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(empty.status(), StatusCode::BAD_REQUEST);

        let rejected = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/token")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"code":"bad-code"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);

        let not_found = app
            .oneshot(Request::builder().uri("/api/unknown").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(not_found.status(), StatusCode::NOT_FOUND);
        assert_eq!(json_body(not_found).await["error"]["code"], "not_found");
    }

    #[tokio::test]
    async fn mutation_routes_reject_missing_and_invalid_tokens() {
        let app = test_router();
        for path in [
            "/api/player/register",
            "/api/player/lang",
            "/api/activities/start",
            "/api/activities/collect",
            "/api/market/sell",
            "/api/market/listings",
            "/api/market/listings/cancel",
            "/api/inventory/claim",
            "/api/city/projects/start",
            "/api/city/treasury/deposit",
            "/api/city/treasury/withdraw",
            "/api/city/tax",
            "/api/crafting/start",
            "/api/crafting/auto-sell",
            "/api/crafting/orders/fulfill",
            "/api/crafting/orders/refresh",
            "/api/crew/recruit",
            "/api/crew/upgrade",
            "/api/crew/assign",
        ] {
            let missing = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(path)
                        .header("content-type", "application/json")
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(missing.status(), StatusCode::UNAUTHORIZED, "{path} without token");

            let invalid = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(path)
                        .header("authorization", "Bearer wrong-token")
                        .header("content-type", "application/json")
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED, "{path} wrong token");
        }
    }

    #[test]
    fn validates_request_ids() {
        assert!(routes::validate_request_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(routes::validate_request_id("req_123").is_ok());
        assert!(routes::validate_request_id("  ").is_err());
        assert!(routes::validate_request_id("bad:colon").is_err());
        assert!(routes::validate_request_id(&"a".repeat(65)).is_err());
    }
}
