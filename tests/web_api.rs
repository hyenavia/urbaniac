//! Integration test untuk Web API Discord Activity (`tests/web_api.rs`).
//! Butuh PostgreSQL (DATABASE_URL) saat dieksekusi lewat `cargo test`.
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rust_decimal::Decimal;
use serenity::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Notify;
use tower::ServiceExt;
use urbaniac::application::voice_boost::VoiceBoostTracker;
use urbaniac::application::{economy, player};
use urbaniac::domain::resources::find_item;
use urbaniac::infrastructure::repositories::inventory_repo;
use urbaniac::web::auth::{AuthUser, DiscordAuthProvider};
use urbaniac::web::error::ApiError;
use urbaniac::web::events::EventHub;
use urbaniac::web::rate_limit::RateLimiter;
use urbaniac::web::{build_router, WebState};
use urbaniac::workers::process_due_batch;

struct TestAuth;

#[async_trait]
impl DiscordAuthProvider for TestAuth {
    async fn exchange_code(&self, code: &str) -> Result<String, ApiError> {
        match code {
            "code-alice" => Ok("token-alice".to_string()),
            "code-bob" => Ok("token-bob".to_string()),
            _ => Err(ApiError::unauthorized("invalid_code", "Invalid code")),
        }
    }

    async fn verify_token(&self, bearer_token: &str) -> Result<AuthUser, ApiError> {
        match bearer_token {
            "token-alice" => Ok(AuthUser {
                user_id: 1001,
                username: "alice".to_string(),
                avatar_url: None,
            }),
            "token-bob" => Ok(AuthUser {
                user_id: 1002,
                username: "bob".to_string(),
                avatar_url: None,
            }),
            _ => Err(ApiError::unauthorized("invalid_token", "Invalid token")),
        }
    }
}

fn router_for(pool: PgPool) -> axum::Router {
    let state = Arc::new(WebState {
        pool,
        notify: Arc::new(Notify::new()),
        activity_duration_pct: 100,
        auth: Arc::new(TestAuth),
        rate_limiter: Arc::new(RateLimiter::default()),
        events: Arc::new(EventHub::default()),
        client_id: "test-client-id".to_string(),
        voice_boost: Arc::new(VoiceBoostTracker::disabled()),
    });
    build_router(state, None)
}

async fn get_json(app: &axum::Router, uri: &str, token: Option<&str>) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder().uri(uri);
    if let Some(tok) = token {
        builder = builder.header("authorization", format!("Bearer {tok}"));
    }
    let resp = app
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap();
    (status, json)
}

async fn post_json(
    app: &axum::Router,
    uri: &str,
    token: Option<&str>,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(tok) = token {
        builder = builder.header("authorization", format!("Bearer {tok}"));
    }
    let resp = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap();
    (status, json)
}

#[sqlx::test(migrations = "./migrations")]
async fn web_api_auth_and_player_isolation(pool: PgPool) {
    let app = router_for(pool.clone());

    // 1. Tanpa atau dengan token salah ditolak 401.
    let (status, _) = get_json(&app, "/api/me", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = get_json(&app, "/api/city", Some("token-invalid")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // 2. Pemain yang belum /start: /api/me -> registered: false; /api/city -> 404.
    let (status, me_unreg) = get_json(&app, "/api/me", Some("token-alice")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me_unreg["registered"], false);
    assert_eq!(me_unreg["discord_user_id"], "1001");

    let (status, _) = get_json(&app, "/api/city", Some("token-alice")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // 3. Daftarkan Alice dan Bob dengan saldo & item berbeda.
    player::register(&pool, 1001, "alice", None).await.unwrap();
    player::register(&pool, 1002, "bob", None).await.unwrap();
    economy::apply(&pool, 1001, Decimal::new(5_000, 2), "test_alice", "ta1")
        .await
        .unwrap();
    {
        let mut tx = pool.begin().await.unwrap();
        let wheat = find_item("wheat").unwrap();
        let wood = find_item("wood").unwrap();
        inventory_repo::add_item(&mut *tx, 1001, wheat, 7, 12).await.unwrap();
        inventory_repo::add_item(&mut *tx, 1002, wood, 9, 12).await.unwrap();
        tx.commit().await.unwrap();
    }

    // 4. Token Alice hanya mengembalikan data Alice.
    let (status, me_a) = get_json(&app, "/api/me", Some("token-alice")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me_a["registered"], true);
    assert_eq!(me_a["discord_user_id"], "1001");
    assert_eq!(me_a["profile"]["balance"], "150.00");

    let (status, city_a) = get_json(&app, "/api/city", Some("token-alice")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(city_a["stage"], "kampung");
    assert_eq!(city_a["wallet_balance"], "150.00");

    let (status, inv_a) = get_json(&app, "/api/inventory", Some("token-alice")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(inv_a["items"].as_array().unwrap().len(), 1);
    assert_eq!(inv_a["items"][0]["item_id"], "wheat");
    assert_eq!(inv_a["items"][0]["quantity"], 7);

    let (status, act_a) = get_json(&app, "/api/activities", Some("token-alice")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(act_a["slots_total"], 2);

    let (status, quote_well) =
        get_json(&app, "/api/city/projects/quote?facility=well", Some("token-alice")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(quote_well["projects"].as_array().unwrap().len(), 1);
    assert_eq!(quote_well["projects"][0]["project"], "well");
    assert_eq!(quote_well["projects"][0]["can_start"], false);

    let (status, quote_bad) =
        get_json(&app, "/api/city/projects/quote?facility=nope", Some("token-alice")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(quote_bad["error"]["code"], "unknown_facility");

    // 5. Token Bob tetap mengembalikan data Bob meski diberi query ?user_id=1001.
    let (status, me_b) = get_json(&app, "/api/me?user_id=1001", Some("token-bob")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me_b["discord_user_id"], "1002");
    assert_eq!(me_b["profile"]["balance"], "100.00");

    let (status, inv_b) = get_json(&app, "/api/inventory?user_id=1001", Some("token-bob")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(inv_b["items"].as_array().unwrap().len(), 1);
    assert_eq!(inv_b["items"][0]["item_id"], "wood");
    assert_eq!(inv_b["items"][0]["quantity"], 9);
}

#[sqlx::test(migrations = "./migrations")]
async fn web_api_mutations_and_idempotency(pool: PgPool) {
    let app = router_for(pool.clone());

    // 1. Register Alice & Bob (idempoten).
    let (status, reg1) = post_json(&app, "/api/player/register", Some("token-alice"), serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reg1["created"], true);
    let (status, reg2) = post_json(&app, "/api/player/register", Some("token-alice"), serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reg2["created"], false);
    post_json(&app, "/api/player/register", Some("token-bob"), serde_json::json!({})).await;

    // 2. Start & collect activity lewat scheduler.
    let (status, run) = post_json(
        &app,
        "/api/activities/start",
        Some("token-alice"),
        serde_json::json!({ "kind": "farming" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(run["kind"], "farming");

    sqlx::query("UPDATE activity_runs SET ends_at = now() - interval '1 second' WHERE status = 'running'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE scheduled_jobs SET run_at = now() - interval '1 second' WHERE status = 'pending'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(process_due_batch(&pool).await.unwrap(), 1);

    let (status, col) = post_json(&app, "/api/activities/collect", Some("token-alice"), serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(col["collected"].as_array().unwrap().len(), 1);

    // 3. Seed Kayu untuk Alice & Bob, lalu uji market/sell idempotency & isolasi key.
    {
        let mut tx = pool.begin().await.unwrap();
        let wood = find_item("wood").unwrap();
        inventory_repo::add_item(&mut *tx, 1001, wood, 15, 12).await.unwrap();
        inventory_repo::add_item(&mut *tx, 1002, wood, 5, 12).await.unwrap();
        // Tambah pending item untuk Alice
        let wheat = find_item("wheat").unwrap();
        sqlx::query(
            "INSERT INTO pending_claims (player_id, item_id, quantity) VALUES (1001, $1, 3)
             ON CONFLICT (player_id, item_id) DO UPDATE SET quantity = pending_claims.quantity + 3",
        )
        .bind(wheat.id)
        .execute(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    let (status, sell1) = post_json(
        &app,
        "/api/market/sell",
        Some("token-alice"),
        serde_json::json!({ "item_id": "wood", "quantity": 2, "request_id": "same-req-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sell1["duplicate"], false);
    assert_eq!(sell1["qty"], 2);
    let bal_after_first = sell1["wallet_balance"].as_str().unwrap().to_string();

    // Retry dengan request_id yang sama: idempoten, saldo tidak bertambah dua kali.
    let (status, sell2) = post_json(
        &app,
        "/api/market/sell",
        Some("token-alice"),
        serde_json::json!({ "item_id": "wood", "quantity": 2, "request_id": "same-req-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sell2["duplicate"], true);
    assert_eq!(sell2["wallet_balance"], bal_after_first);

    // Bob pakai request_id persis sama tidak bertabrakan dengan Alice.
    let (status, sell_bob) = post_json(
        &app,
        "/api/market/sell",
        Some("token-bob"),
        serde_json::json!({ "item_id": "wood", "quantity": 2, "request_id": "same-req-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sell_bob["duplicate"], false);

    // 4. Klaim Pending.
    let (status, claimed) = post_json(&app, "/api/inventory/claim", Some("token-alice"), serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!claimed["claimed"].as_array().unwrap().is_empty());

    // 5. Treasury deposit/withdraw & tax.
    let (status, dep) = post_json(
        &app,
        "/api/city/treasury/deposit",
        Some("token-alice"),
        serde_json::json!({ "amount": "20.00", "request_id": "dep-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(dep["treasury"], "20.00");

    let (status, wd) = post_json(
        &app,
        "/api/city/treasury/withdraw",
        Some("token-alice"),
        serde_json::json!({ "amount": "5.00", "request_id": "wd-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(wd["treasury"], "15.00");

    let (status, tax) = post_json(
        &app,
        "/api/city/tax",
        Some("token-alice"),
        serde_json::json!({ "tax_rate_pct": 15 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(tax["tax_rate_pct"], 15);

    // 6. Mulai proyek Well, lalu proyek kedua ditolak 409 saat masih berjalan.
    let (status, proj) = post_json(
        &app,
        "/api/city/projects/start",
        Some("token-alice"),
        serde_json::json!({ "facility": "well", "request_id": "proj-well-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(proj["project"], "well");

    let (status, dup_proj) = post_json(
        &app,
        "/api/city/projects/start",
        Some("token-alice"),
        serde_json::json!({ "facility": "granary", "request_id": "proj-gran-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(dup_proj["error"]["code"], "project_already_running");
}

#[sqlx::test(migrations = "./migrations")]
async fn web_api_phase2_to_9_endpoints(pool: PgPool) {
    let app = router_for(pool.clone());

    post_json(&app, "/api/player/register", Some("token-alice"), serde_json::json!({})).await;
    economy::apply(&pool, 1001, Decimal::new(50_000, 2), "grant", "g_p4")
        .await
        .unwrap();
    {
        let mut tx = pool.begin().await.unwrap();
        let wheat = find_item("wheat").unwrap();
        inventory_repo::add_item(&mut *tx, 1001, wheat, 20, 12).await.unwrap();
        tx.commit().await.unwrap();
    }

    // 1. Language endpoint always returns English
    let (status, lang_resp) = post_json(
        &app,
        "/api/player/lang",
        Some("token-alice"),
        serde_json::json!({ "lang": "id" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(lang_resp["lang"], "en");

    // 2. Market & Broker listing
    let (status, mkt) = get_json(&app, "/api/market", Some("token-alice")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!mkt["prices"].as_array().unwrap().is_empty());

    let (status, lst) = post_json(
        &app,
        "/api/market/listings",
        Some("token-alice"),
        serde_json::json!({
            "item_id": "wheat",
            "quantity": 5,
            "target_unit_price": "4.50",
            "duration": "2h",
            "request_id": "lst-1"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let listing_id = lst["id"].as_i64().unwrap();

    let (status, cancel) = post_json(
        &app,
        "/api/market/listings/cancel",
        Some("token-alice"),
        serde_json::json!({ "listing_id": listing_id }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cancel["stored"], 5);

    // 3. Crafting & Auto-sell
    let (status, craft_ov) = get_json(&app, "/api/crafting", Some("token-alice")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!craft_ov["recipes"].as_array().unwrap().is_empty());

    let (status, auto_res) =
        post_json(&app, "/api/crafting/auto-sell", Some("token-alice"), serde_json::json!({}))
            .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(auto_res["auto_sell"], true);

    let (status, craft_run) = post_json(
        &app,
        "/api/crafting/start",
        Some("token-alice"),
        serde_json::json!({ "recipe_id": "flour", "batch_qty": 2 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(craft_run["recipe_id"], "flour");

    // 4. Crew recruit, assign, upgrade
    let (status, crew_ov) = get_json(&app, "/api/crew", Some("token-alice")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(crew_ov["members"].as_array().unwrap().len(), 0);

    let (status, recruited) = post_json(
        &app,
        "/api/crew/recruit",
        Some("token-alice"),
        serde_json::json!({ "request_id": "crew-rec-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let crew_id = recruited["id"].as_i64().unwrap();

    let (status, assigned) = post_json(
        &app,
        "/api/crew/assign",
        Some("token-alice"),
        serde_json::json!({ "crew_id": crew_id, "activity": "farming", "auto_enabled": true }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(assigned["assigned"], "farming");
    assert_eq!(assigned["auto_enabled"], true);

    let (status, upgraded) = post_json(
        &app,
        "/api/crew/upgrade",
        Some("token-alice"),
        serde_json::json!({ "crew_id": crew_id, "request_id": "crew-upg-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(upgraded["level"], 2);
}
