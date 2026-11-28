use crate::application::activities::{self, ActivityError};
use crate::application::city::{self, CityError};
use crate::application::crafting::{self, CraftError};
use crate::application::crew::{self, CrewError, CrewView};
use crate::application::economy::{self, SaleOutcome, SellError, SellQty};
use crate::application::market::{self, MarketError};
use crate::application::{inventory, player};
use crate::bot::i18n_errors;
use crate::domain::activities::{def as activity_def, ActivityKind};
use crate::domain::city::ProjectId;
use crate::domain::crafting::{
    craft_xp_per_unit, max_craftable_batches, RecipeTier, RECIPES,
};
use crate::domain::crew::{recruit_cost, MAX_CREW_LEVEL, MAX_CREW_SLOTS};
use crate::domain::i18n::{
    activity_name, event_bonus_hint, item_name, project_desc, project_name, recipe_name,
    recipe_tier_name, season_bonus_hint, season_name, stage_era, stage_name, world_event_name,
    Lang,
};
use crate::domain::market::{
    ListingDuration, BROKER_COMMISSION_PCT, LISTING_FEE_PCT, MAX_ACTIVE_LISTINGS,
};
use crate::domain::player::xp_to_next;
use crate::domain::resources::find_item;
use crate::infrastructure::repositories::wallet_repo::WalletError;
use crate::web::auth::AuthUser;
use crate::web::dto::*;
use crate::web::error::ApiError;
use crate::web::events::EventHub;
use crate::web::WebState;
use axum::extract::{Query, State};
use axum::http::{header, HeaderName};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;
use futures_util::stream;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;

const TOKEN_RATE_LIMIT_MAX: u32 = 20;
const TOKEN_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

fn fmt_bac(d: Decimal) -> String {
    format!("{:.2}", d.round_dp(2))
}

fn clean_msg(raw: String) -> String {
    raw.replace("**", "").replace('`', "")
}

pub(crate) fn validate_request_id(raw: &str) -> Result<&str, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.len() > 64
        || !trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(ApiError::bad_request(
            "invalid_request_id",
            "request_id must be 1..64 ASCII alphanumeric, '-' or '_' characters",
        ));
    }
    Ok(trimmed)
}

async fn require_registered_lang(state: &WebState, player_id: i64) -> Result<Lang, ApiError> {
    let Some(p) = player::get_player(&state.pool, player_id).await? else {
        return Err(ApiError::not_found(
            "not_registered",
            "Player profile not found. Register first in the Urbaniac Activity.",
        ));
    };
    Ok(p.language())
}

fn map_activity_error(err: ActivityError, lang: Lang) -> ApiError {
    let msg = clean_msg(i18n_errors::activity_error(&err, lang));
    match err {
        ActivityError::NotRegistered => ApiError::not_found("not_registered", msg),
        ActivityError::Locked(_) => ApiError::conflict("activity_locked", msg),
        ActivityError::NoFreeSlot(_) => ApiError::conflict("no_free_slot", msg),
        ActivityError::Db(e) => ApiError::Internal(e.into()),
    }
}

fn map_city_error(err: CityError, lang: Lang) -> ApiError {
    let msg = clean_msg(i18n_errors::city_error(&err, lang));
    match err {
        CityError::NotRegistered => ApiError::not_found("not_registered", msg),
        CityError::ProjectAlreadyRunning(_) => ApiError::conflict("project_already_running", msg),
        CityError::AlreadyBuilt(_) => ApiError::conflict("already_built", msg),
        CityError::StageTooLow(_) => ApiError::conflict("stage_too_low", msg),
        CityError::MissingPrerequisites => ApiError::conflict("missing_prerequisites", msg),
        CityError::StageAlreadyAdvanced => ApiError::conflict("stage_already_advanced", msg),
        CityError::InsufficientBac { .. } => ApiError::conflict("insufficient_bac", msg),
        CityError::InsufficientTreasury { .. } => ApiError::conflict("insufficient_treasury", msg),
        CityError::InsufficientItem { .. } => ApiError::conflict("insufficient_item", msg),
        CityError::InvalidAmount => ApiError::bad_request("invalid_amount", msg),
        CityError::InvalidTaxRate => ApiError::bad_request("invalid_tax_rate", msg),
        CityError::Wallet(WalletError::NotFound) => ApiError::not_found("not_registered", msg),
        CityError::Wallet(WalletError::InsufficientFunds) => {
            ApiError::conflict("insufficient_bac", msg)
        }
        CityError::Wallet(WalletError::Db(e)) | CityError::Db(e) => ApiError::Internal(e.into()),
    }
}

fn map_sell_error(err: SellError, lang: Lang) -> ApiError {
    let msg = clean_msg(i18n_errors::sell_error(&err, lang));
    match err {
        SellError::UnknownItem => ApiError::bad_request("unknown_item", msg),
        SellError::InvalidQty => ApiError::bad_request("invalid_qty", msg),
        SellError::NotEnough { .. } => ApiError::conflict("insufficient_item", msg),
        SellError::Wallet(WalletError::NotFound) => ApiError::not_found("not_registered", msg),
        SellError::Wallet(WalletError::InsufficientFunds) => {
            ApiError::conflict("insufficient_bac", msg)
        }
        SellError::Wallet(WalletError::Db(e)) | SellError::Db(e) => ApiError::Internal(e.into()),
    }
}

fn map_market_error(err: MarketError, lang: Lang) -> ApiError {
    let msg = clean_msg(i18n_errors::market_error(&err, lang));
    match err {
        MarketError::NotRegistered => ApiError::not_found("not_registered", msg),
        MarketError::UnknownItem => ApiError::bad_request("unknown_item", msg),
        MarketError::InvalidQty => ApiError::bad_request("invalid_qty", msg),
        MarketError::InvalidPrice => ApiError::bad_request("invalid_price", msg),
        MarketError::TooManyListings(_) => ApiError::conflict("too_many_listings", msg),
        MarketError::InsufficientItem { .. } => ApiError::conflict("insufficient_item", msg),
        MarketError::InsufficientBacForFee { .. } => ApiError::conflict("insufficient_bac", msg),
        MarketError::ListingNotFound => ApiError::not_found("listing_not_found", msg),
        MarketError::Wallet(WalletError::NotFound) => ApiError::not_found("not_registered", msg),
        MarketError::Wallet(WalletError::InsufficientFunds) => {
            ApiError::conflict("insufficient_bac", msg)
        }
        MarketError::Wallet(WalletError::Db(e)) | MarketError::Db(e) => {
            ApiError::Internal(e.into())
        }
    }
}

fn map_craft_error(err: CraftError, lang: Lang) -> ApiError {
    let msg = clean_msg(i18n_errors::craft_error(&err, lang));
    match err {
        CraftError::NotRegistered => ApiError::not_found("not_registered", msg),
        CraftError::UnknownRecipe => ApiError::bad_request("unknown_recipe", msg),
        CraftError::InvalidQty => ApiError::bad_request("invalid_qty", msg),
        CraftError::TooManyCrafts(_) => ApiError::conflict("too_many_crafts", msg),
        CraftError::MissingIngredient { .. } => ApiError::conflict("missing_ingredient", msg),
        CraftError::OrderNotFound => ApiError::not_found("order_not_found", msg),
        CraftError::InsufficientOrderItem { .. } => ApiError::conflict("insufficient_item", msg),
        CraftError::OrderRefreshCooldown(_) => ApiError::conflict("order_refresh_cooldown", msg),
        CraftError::Wallet(WalletError::NotFound) => ApiError::not_found("not_registered", msg),
        CraftError::Wallet(WalletError::InsufficientFunds) => {
            ApiError::conflict("insufficient_bac", msg)
        }
        CraftError::Wallet(WalletError::Db(e)) | CraftError::Db(e) => ApiError::Internal(e.into()),
    }
}

fn map_crew_error(err: CrewError, lang: Lang) -> ApiError {
    let msg = clean_msg(i18n_errors::crew_error(&err, lang));
    match err {
        CrewError::NotRegistered => ApiError::not_found("not_registered", msg),
        CrewError::CrewFull(_) => ApiError::conflict("crew_full", msg),
        CrewError::NotFound => ApiError::not_found("crew_not_found", msg),
        CrewError::MaxLevel(_) => ApiError::conflict("max_level", msg),
        CrewError::ActivityLocked(_) => ApiError::conflict("activity_locked", msg),
        CrewError::InsufficientBac { .. } => ApiError::conflict("insufficient_bac", msg),
        CrewError::Wallet(WalletError::NotFound) => ApiError::not_found("not_registered", msg),
        CrewError::Wallet(WalletError::InsufficientFunds) => {
            ApiError::conflict("insufficient_bac", msg)
        }
        CrewError::Wallet(WalletError::Db(e)) | CrewError::Db(e) => ApiError::Internal(e.into()),
    }
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

pub async fn app_config(State(state): State<Arc<WebState>>) -> Json<AppConfigResponse> {
    Json(AppConfigResponse {
        discord_client_id: state.client_id.clone(),
    })
}

pub async fn events_stream(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
) -> Result<impl IntoResponse, ApiError> {
    let (rx, guard) = state.events.subscribe(auth.user_id)?;
    let user_id = auth.user_id;

    let stream = stream::unfold(
        Some((rx, guard, false)),
        move |state_opt| async move {
            let (mut rx, guard, sent_hello) = state_opt?;
            if !sent_hello {
                let ev = Event::default()
                    .event("connected")
                    .data(r#"{"kind":"connected","message":null}"#);
                return Some((Ok::<Event, Infallible>(ev), Some((rx, guard, true))));
            }
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        if !EventHub::is_visible_to(&msg, user_id) {
                            continue;
                        }
                        if let Ok(ev) = Event::default().event(msg.kind).json_data(&msg) {
                            return Some((Ok(ev), Some((rx, guard, true))));
                        }
                    }
                    Err(RecvError::Lagged(_)) => {
                        let ev = Event::default()
                            .event("state_sync")
                            .data(r#"{"kind":"state_sync","message":null}"#);
                        return Some((Ok(ev), Some((rx, guard, true))));
                    }
                    Err(RecvError::Closed) => return None,
                }
            }
        },
    );

    let sse = Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(20))
            .text("ping"),
    );

    Ok((
        [
            (header::CACHE_CONTROL, "no-cache"),
            (HeaderName::from_static("x-accel-buffering"), "no"),
        ],
        sse,
    ))
}

pub async fn token_exchange(
    State(state): State<Arc<WebState>>,
    Json(req): Json<TokenExchangeRequest>,
) -> Result<Json<TokenExchangeResponse>, ApiError> {
    if !state
        .rate_limiter
        .check("token:global", TOKEN_RATE_LIMIT_MAX, TOKEN_RATE_LIMIT_WINDOW)
    {
        return Err(ApiError::too_many_requests(
            "rate_limited",
            "Too many token requests; please retry shortly",
        ));
    }

    let code = req.code.trim();
    if code.is_empty() || code.len() > 512 {
        return Err(ApiError::bad_request(
            "invalid_code",
            "Authorization code is missing or invalid",
        ));
    }

    let access_token = state.auth.exchange_code(code).await?;
    Ok(Json(TokenExchangeResponse { access_token }))
}

pub async fn register_player(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
) -> Result<Json<RegisterResponse>, ApiError> {
    let (_, created) = player::register(
        &state.pool,
        auth.user_id,
        &auth.username,
        auth.avatar_url.as_deref(),
    )
    .await?;
    Ok(Json(RegisterResponse {
        created,
        discord_user_id: auth.user_id.to_string(),
    }))
}

pub async fn set_player_lang(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(_req): Json<SetLangRequest>,
) -> Result<Json<SetLangResponse>, ApiError> {
    require_registered_lang(&state, auth.user_id).await?;
    Ok(Json(SetLangResponse {
        lang: Lang::En.as_str(),
        label: Lang::En.label(),
    }))
}

pub async fn me(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
) -> Result<Json<MeResponse>, ApiError> {
    let profile = match player::get_player_profile(&state.pool, auth.user_id).await? {
        Some(pv) => {
            let lang = pv.player.language();
            Some(MeProfileDto {
                level: pv.player.level,
                xp: pv.player.xp,
                xp_needed: xp_to_next(pv.player.level),
                balance: fmt_bac(pv.balance),
                city_stage: pv.city_stage.as_str(),
                city_stage_name: stage_name(pv.city_stage, lang),
                lang: lang.as_str(),
                slots_used: pv.inventory.slots_used,
                slot_capacity: pv.inventory.slot_capacity,
            })
        }
        None => None,
    };

    Ok(Json(MeResponse {
        registered: profile.is_some(),
        discord_user_id: auth.user_id.to_string(),
        username: auth.username,
        avatar_url: auth.avatar_url,
        profile,
        voice_boost_active: state.voice_boost.is_boosted(auth.user_id),
        voice_boost_speed_pct: state.voice_boost.speed_bonus_pct(auth.user_id),
        voice_boost_xp_pct: state.voice_boost.xp_bonus_pct(auth.user_id),
    }))
}

pub async fn get_city(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
) -> Result<Json<CityResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let ov = city::overview(&state.pool, auth.user_id).await?;

    let facilities = ov
        .facilities
        .into_iter()
        .map(|f| FacilityDto {
            id: f.id.as_str(),
            name: project_name(f.id, lang),
            level: f.level,
        })
        .collect();

    let running_project = ov.running_project.map(|rp| RunningProjectDto {
        id: rp.id,
        project: rp.project.as_str(),
        name: project_name(rp.project, lang),
        ends_at: rp.ends_at.to_rfc3339(),
    });

    Ok(Json(CityResponse {
        stage: ov.stage.as_str(),
        stage_name: stage_name(ov.stage, lang),
        era: stage_era(ov.stage, lang),
        treasury: fmt_bac(ov.treasury),
        wallet_balance: fmt_bac(ov.wallet_balance),
        tax_rate_pct: ov.tax_rate_pct,
        welfare: ov.welfare,
        effectiveness_pct: ov.effectiveness_pct,
        yield_bonus_pct: ov.yield_bonus_pct,
        est_tax_income: fmt_bac(ov.est_tax_income),
        est_expense: fmt_bac(ov.est_expense),
        assigned_crews: ov.assigned_crews,
        facilities,
        running_project,
        server_time: Utc::now().to_rfc3339(),
    }))
}

#[derive(Deserialize)]
pub struct QuoteQuery {
    pub facility: Option<String>,
}

fn to_quote_dto(q: city::ProjectQuoteView, lang: Lang) -> ProjectQuoteDto {
    ProjectQuoteDto {
        project: q.project.as_str(),
        name: project_name(q.project, lang),
        description: project_desc(q.project, lang),
        current_level: q.current_level,
        next_level: q.next_level,
        max_level: q.max_level,
        min_stage: q.min_stage.as_str(),
        min_stage_name: stage_name(q.min_stage, lang),
        cost_bac: fmt_bac(q.cost_bac),
        available_bac: fmt_bac(q.available_bac),
        duration_secs: q.duration_secs,
        items: q
            .items
            .into_iter()
            .map(|it| ProjectQuoteItemDto {
                item_id: it.item_id,
                name: item_name(it.item_id, lang),
                need: it.need,
                have: it.have,
            })
            .collect(),
        can_start: q.can_start,
        block_reason: q.block_reason,
    }
}

pub async fn city_projects_quote(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Query(query): Query<QuoteQuery>,
) -> Result<Json<ProjectQuoteResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let quotes = match query.facility.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => {
            let pid = ProjectId::parse(raw).ok_or_else(|| {
                ApiError::bad_request("unknown_facility", format!("Unknown facility: {raw}"))
            })?;
            vec![city::quote(&state.pool, auth.user_id, pid, state.activity_duration_pct).await?]
        }
        None => city::quote_all(&state.pool, auth.user_id, state.activity_duration_pct).await?,
    };

    Ok(Json(ProjectQuoteResponse {
        projects: quotes.into_iter().map(|q| to_quote_dto(q, lang)).collect(),
    }))
}

pub async fn city_projects_start(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(req): Json<StartProjectRequest>,
) -> Result<Json<RunningProjectDto>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let rid = validate_request_id(&req.request_id)?;
    let pid = ProjectId::parse(req.facility.trim()).ok_or_else(|| {
        ApiError::bad_request("unknown_facility", format!("Unknown facility: {}", req.facility))
    })?;
    let key = format!("proj:{}:{rid}", auth.user_id);

    let rp = city::start_project(
        &state.pool,
        auth.user_id,
        pid,
        state.activity_duration_pct,
        &key,
    )
    .await
    .map_err(|e| map_city_error(e, lang))?;
    state.notify.notify_one();

    Ok(Json(RunningProjectDto {
        id: rp.id,
        project: rp.project.as_str(),
        name: project_name(rp.project, lang),
        ends_at: rp.ends_at.to_rfc3339(),
    }))
}

pub async fn city_treasury_deposit(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(req): Json<TreasuryTransferRequest>,
) -> Result<Json<TreasuryBalanceResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let rid = validate_request_id(&req.request_id)?;
    let amount = req
        .amount
        .trim()
        .parse::<Decimal>()
        .map_err(|_| ApiError::bad_request("invalid_amount", "Invalid Bac amount"))?;
    let key = format!("tdep:{}:{rid}", auth.user_id);

    let (wallet, treasury) = city::deposit_to_treasury(&state.pool, auth.user_id, amount, &key)
        .await
        .map_err(|e| map_city_error(e, lang))?;

    Ok(Json(TreasuryBalanceResponse {
        wallet_balance: fmt_bac(wallet),
        treasury: fmt_bac(treasury),
    }))
}

pub async fn city_treasury_withdraw(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(req): Json<TreasuryTransferRequest>,
) -> Result<Json<TreasuryBalanceResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let rid = validate_request_id(&req.request_id)?;
    let amount = req
        .amount
        .trim()
        .parse::<Decimal>()
        .map_err(|_| ApiError::bad_request("invalid_amount", "Invalid Bac amount"))?;
    let key = format!("twdr:{}:{rid}", auth.user_id);

    let (wallet, treasury) = city::withdraw_from_treasury(&state.pool, auth.user_id, amount, &key)
        .await
        .map_err(|e| map_city_error(e, lang))?;

    Ok(Json(TreasuryBalanceResponse {
        wallet_balance: fmt_bac(wallet),
        treasury: fmt_bac(treasury),
    }))
}

pub async fn city_set_tax(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(req): Json<SetTaxRequest>,
) -> Result<Json<SetTaxResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let pct = city::set_tax_rate(&state.pool, auth.user_id, req.tax_rate_pct)
        .await
        .map_err(|e| map_city_error(e, lang))?;
    Ok(Json(SetTaxResponse { tax_rate_pct: pct }))
}

pub async fn get_inventory(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
) -> Result<Json<InventoryResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let inv = inventory::view(&state.pool, auth.user_id).await?;

    let mut items = Vec::with_capacity(inv.lines.len());
    for line in inv.lines {
        let unit_price = economy::sell_unit_price(&state.pool, auth.user_id, line.item).await?;
        items.push(InventoryItemDto {
            item_id: line.item.id,
            name: item_name(line.item.id, lang),
            rarity: line.item.rarity.to_string(),
            quantity: line.quantity,
            npc_unit_price: fmt_bac(unit_price),
        });
    }

    let pending = inv
        .pending
        .into_iter()
        .map(|line| PendingItemDto {
            item_id: line.item.id,
            name: item_name(line.item.id, lang),
            rarity: line.item.rarity.to_string(),
            quantity: line.quantity,
        })
        .collect();

    Ok(Json(InventoryResponse {
        slots_used: inv.slots_used,
        slot_capacity: inv.slot_capacity,
        items,
        pending,
    }))
}

pub async fn inventory_claim(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
) -> Result<Json<ClaimPendingResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let results = inventory::claim_pending(&state.pool, auth.user_id).await?;
    let claimed = results
        .into_iter()
        .map(|r| {
            let name = item_name(&r.item_id, lang);
            ClaimedPendingItemDto {
                item_id: r.item_id,
                name,
                claimed: r.claimed,
                remaining: r.remaining,
            }
        })
        .collect();
    Ok(Json(ClaimPendingResponse { claimed }))
}

pub async fn market_sell(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(req): Json<MarketSellRequest>,
) -> Result<Json<MarketSellResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let rid = validate_request_id(&req.request_id)?;
    let item_id = req.item_id.trim();
    let sell_qty = if req.sell_all == Some(true) {
        SellQty::All
    } else if let Some(qty) = req.quantity {
        if qty <= 0 {
            return Err(ApiError::bad_request("invalid_qty", "Quantity must be greater than 0"));
        }
        SellQty::Exact(qty)
    } else {
        return Err(ApiError::bad_request(
            "invalid_qty",
            "Specify quantity or set sell_all=true",
        ));
    };

    let key = format!("sell:{}:{rid}", auth.user_id);
    let outcome = economy::sell_to_npc(&state.pool, auth.user_id, item_id, sell_qty, &key)
        .await
        .map_err(|e| map_sell_error(e, lang))?;

    let resp = match outcome {
        SaleOutcome::Sold { item, qty, total, balance } => MarketSellResponse {
            duplicate: false,
            item_id: item.id,
            name: item_name(item.id, lang),
            qty,
            total_bac: fmt_bac(total),
            wallet_balance: fmt_bac(balance),
        },
        SaleOutcome::Duplicate => {
            let item = find_item(item_id).ok_or_else(|| {
                ApiError::bad_request("unknown_item", "Unknown item")
            })?;
            let balance = economy::balance(&state.pool, auth.user_id).await?;
            MarketSellResponse {
                duplicate: true,
                item_id: item.id,
                name: item_name(item.id, lang),
                qty: 0,
                total_bac: "0.00".to_string(),
                wallet_balance: fmt_bac(balance),
            }
        }
    };

    Ok(Json(resp))
}

pub async fn get_market(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
) -> Result<Json<MarketResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let ov = market::overview(&state.pool, auth.user_id).await?;

    let calendar = CalendarDto {
        day: ov.calendar.day,
        season: ov.calendar.season.as_str(),
        season_name: season_name(ov.calendar.season, lang),
        season_hint: season_bonus_hint(ov.calendar.season, lang),
        active_event: ov.calendar.active_event.map(|e| e.as_str()),
        active_event_name: ov.calendar.active_event.map(|e| world_event_name(e, lang)),
        event_hint: ov.calendar.active_event.map(|e| event_bonus_hint(e, lang)),
        trend_item_id: ov.calendar.trend_item_id,
        trend_item_name: ov.calendar.trend_item_id.map(|id| item_name(id, lang)),
    };

    let prices = ov
        .prices
        .into_iter()
        .map(|p| MarketPriceDto {
            item_id: p.item.id,
            name: item_name(p.item.id, lang),
            rarity: p.item.rarity.to_string(),
            base_price: fmt_bac(p.base_price),
            current_price: fmt_bac(p.current_price),
            multiplier: format!("{:.2}", p.multiplier.round_dp(2)),
        })
        .collect();

    let listings = ov
        .listings
        .into_iter()
        .map(|l| BrokerListingDto {
            id: l.id,
            item_id: l.item.id,
            name: item_name(l.item.id, lang),
            quantity: l.quantity,
            target_unit_price: fmt_bac(l.target_unit_price),
            listing_fee: fmt_bac(l.listing_fee),
            expires_at: l.expires_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(MarketResponse {
        wallet_balance: fmt_bac(ov.wallet_balance),
        calendar,
        prices,
        listings,
        max_listings: MAX_ACTIVE_LISTINGS,
        fee_pct: LISTING_FEE_PCT,
        commission_pct: BROKER_COMMISSION_PCT,
        server_time: Utc::now().to_rfc3339(),
    }))
}

pub async fn create_market_listing(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(req): Json<CreateListingRequest>,
) -> Result<Json<BrokerListingDto>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let rid = validate_request_id(&req.request_id)?;
    let target = req
        .target_unit_price
        .trim()
        .parse::<Decimal>()
        .map_err(|_| ApiError::bad_request("invalid_price", "Invalid target unit price"))?;
    let duration = ListingDuration::parse(req.duration.trim()).ok_or_else(|| {
        ApiError::bad_request("invalid_duration", "Duration must be '2h', '6h', or '24h'")
    })?;
    let key = format!("blist:{}:{rid}", auth.user_id);

    let listing = market::create_listing(
        &state.pool,
        auth.user_id,
        req.item_id.trim(),
        req.quantity,
        target,
        duration,
        state.activity_duration_pct,
        &key,
    )
    .await
    .map_err(|e| map_market_error(e, lang))?;
    state.notify.notify_one();

    Ok(Json(BrokerListingDto {
        id: listing.id,
        item_id: listing.item.id,
        name: item_name(listing.item.id, lang),
        quantity: listing.quantity,
        target_unit_price: fmt_bac(listing.target_unit_price),
        listing_fee: fmt_bac(listing.listing_fee),
        expires_at: listing.expires_at.to_rfc3339(),
    }))
}

pub async fn cancel_market_listing(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(req): Json<CancelListingRequest>,
) -> Result<Json<CancelListingResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let (item, stored, pending) =
        market::cancel_listing(&state.pool, auth.user_id, req.listing_id)
            .await
            .map_err(|e| map_market_error(e, lang))?;
    Ok(Json(CancelListingResponse {
        item_id: item.id,
        name: item_name(item.id, lang),
        stored,
        pending,
    }))
}

pub async fn get_crafting(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
) -> Result<Json<CraftingResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let ov = crafting::overview(&state.pool, auth.user_id).await?;
    let inv = inventory::view(&state.pool, auth.user_id).await?;
    let stock: HashMap<&str, i64> = inv
        .lines
        .iter()
        .map(|l| (l.item.id, l.quantity))
        .collect();

    let running = ov
        .running
        .into_iter()
        .map(|r| CraftRunDto {
            id: r.id,
            recipe_id: r.recipe.id,
            name: recipe_name(r.recipe.id, lang),
            quantity: r.quantity,
            auto_sell: r.auto_sell,
            ends_at: r.ends_at.to_rfc3339(),
        })
        .collect();

    let orders = ov
        .orders
        .into_iter()
        .map(|o| {
            let have = stock.get(o.item.id).copied().unwrap_or(0);
            NpcOrderDto {
                id: o.id,
                item_id: o.item.id,
                name: item_name(o.item.id, lang),
                quantity: o.quantity,
                have,
                reward_bac: fmt_bac(o.reward_bac),
                rep_xp: o.rep_xp,
                can_fulfill: have >= o.quantity,
            }
        })
        .collect();

    let recipes = RECIPES
        .iter()
        .map(|r| {
            let tier = match r.tier {
                RecipeTier::Processed => "processed",
                RecipeTier::Finished => "finished",
            };
            RecipeDto {
                id: r.id,
                name: recipe_name(r.id, lang),
                tier,
                tier_name: recipe_tier_name(r.tier, lang),
                output_item_id: r.output_item_id,
                output_qty: r.output_qty,
                duration_secs: (r.duration_secs * state.activity_duration_pct.max(1) / 100)
                    .max(1),
                xp_per_unit: craft_xp_per_unit(r.tier),
                inputs: r
                    .inputs
                    .iter()
                    .map(|(id, need)| RecipeInputDto {
                        item_id: id,
                        name: item_name(id, lang),
                        need: *need,
                        have: stock.get(id).copied().unwrap_or(0),
                    })
                    .collect(),
                max_batches: max_craftable_batches(r, &stock),
            }
        })
        .collect();

    Ok(Json(CraftingResponse {
        max_concurrent: ov.max_concurrent,
        rep_level: ov.rep_level,
        rep_xp: ov.rep_xp,
        rep_xp_needed: ov.rep_xp_needed,
        rep_bonus_pct: ov.rep_bonus_pct,
        completed_orders: ov.completed_orders,
        auto_sell: ov.auto_sell,
        running,
        orders,
        recipes,
        server_time: Utc::now().to_rfc3339(),
    }))
}

pub async fn start_crafting(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(req): Json<StartCraftRequest>,
) -> Result<Json<CraftRunDto>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let batch_qty = req.batch_qty.unwrap_or(1);
    let voice_spd = state.voice_boost.speed_bonus_pct(auth.user_id);
    let run = crafting::start_craft_batch_with_bonus(
        &state.pool,
        auth.user_id,
        req.recipe_id.trim(),
        batch_qty,
        state.activity_duration_pct,
        voice_spd,
    )
    .await
    .map_err(|e| map_craft_error(e, lang))?;
    state.notify.notify_one();

    Ok(Json(CraftRunDto {
        id: run.id,
        recipe_id: run.recipe.id,
        name: recipe_name(run.recipe.id, lang),
        quantity: run.quantity,
        auto_sell: run.auto_sell,
        ends_at: run.ends_at.to_rfc3339(),
    }))
}

pub async fn toggle_crafting_auto_sell(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
) -> Result<Json<ToggleAutoSellResponse>, ApiError> {
    require_registered_lang(&state, auth.user_id).await?;
    let auto_sell = crafting::toggle_auto_sell(&state.pool, auth.user_id).await?;
    Ok(Json(ToggleAutoSellResponse { auto_sell }))
}

pub async fn fulfill_crafting_order(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(req): Json<FulfillOrderRequest>,
) -> Result<Json<FulfillOrderResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let xp_bonus = state.voice_boost.xp_bonus_pct(auth.user_id);
    let sum = crafting::fulfill_order_with_xp_bonus(&state.pool, auth.user_id, req.order_id, xp_bonus)
        .await
        .map_err(|e| map_craft_error(e, lang))?;
    Ok(Json(FulfillOrderResponse {
        item_id: sum.item.id,
        name: item_name(sum.item.id, lang),
        quantity: sum.quantity,
        reward_bac: fmt_bac(sum.reward_bac),
        rep_xp_gain: sum.rep_xp_gain,
        old_rep_level: sum.old_rep_level,
        new_rep_level: sum.new_rep_level,
    }))
}

pub async fn refresh_crafting_orders(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
) -> Result<Json<OkResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    crafting::refresh_orders(&state.pool, auth.user_id)
        .await
        .map_err(|e| map_craft_error(e, lang))?;
    Ok(Json(OkResponse { ok: true }))
}

fn to_crew_dto(c: CrewView, lang: Lang) -> CrewMemberDto {
    CrewMemberDto {
        id: c.id,
        name: c.name,
        rarity: c.rarity.to_string(),
        specialty: c.specialty.as_str(),
        specialty_name: activity_name(c.specialty, lang),
        level: c.level,
        assigned: c.assigned.map(|a| a.as_str()),
        assigned_name: c.assigned.map(|a| activity_name(a, lang)),
        auto_enabled: c.auto_enabled,
        efficiency_pct: c.bonus.efficiency_pct,
        yield_pct: c.bonus.yield_pct,
        luck_pct: c.bonus.luck_pct,
        upgrade_cost: c.upgrade_cost.map(fmt_bac),
    }
}

pub async fn get_crew(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
) -> Result<Json<CrewResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let list = crew::list(&state.pool, auth.user_id).await?;
    Ok(Json(CrewResponse {
        max_slots: MAX_CREW_SLOTS,
        max_level: MAX_CREW_LEVEL,
        recruit_cost: fmt_bac(recruit_cost()),
        members: list.into_iter().map(|c| to_crew_dto(c, lang)).collect(),
    }))
}

pub async fn recruit_crew(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(req): Json<RecruitCrewRequest>,
) -> Result<Json<CrewMemberDto>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let rid = validate_request_id(&req.request_id)?;
    let key = format!("crec:{}:{rid}", auth.user_id);
    let member = crew::recruit(&state.pool, auth.user_id, &key)
        .await
        .map_err(|e| map_crew_error(e, lang))?;
    Ok(Json(to_crew_dto(member, lang)))
}

pub async fn upgrade_crew(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(req): Json<UpgradeCrewRequest>,
) -> Result<Json<CrewMemberDto>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let rid = validate_request_id(&req.request_id)?;
    let key = format!("cupg:{}:{rid}", auth.user_id);
    let member = crew::upgrade(&state.pool, auth.user_id, req.crew_id, &key)
        .await
        .map_err(|e| map_crew_error(e, lang))?;
    Ok(Json(to_crew_dto(member, lang)))
}

pub async fn assign_crew(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(req): Json<AssignCrewRequest>,
) -> Result<Json<CrewMemberDto>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let activity = match req.activity.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => Some(ActivityKind::parse(raw).ok_or_else(|| {
            ApiError::bad_request("unknown_activity", format!("Unknown activity: {raw}"))
        })?),
        None => None,
    };
    let member = crew::assign(
        &state.pool,
        auth.user_id,
        req.crew_id,
        activity,
        req.auto_enabled,
    )
    .await
    .map_err(|e| map_crew_error(e, lang))?;
    Ok(Json(to_crew_dto(member, lang)))
}

pub async fn get_activities(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
) -> Result<Json<ActivitiesResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let ov = activities::overview(&state.pool, auth.user_id).await?;

    let runs = ov
        .runs
        .into_iter()
        .map(|r| ActivityRunDto {
            id: r.id,
            kind: r.kind.as_str(),
            name: activity_name(r.kind, lang),
            status: match r.status {
                activities::RunStatus::Running => "running",
                activities::RunStatus::Ready => "ready",
            },
            ends_at: r.ends_at.to_rfc3339(),
        })
        .collect();

    let progress = ov
        .progress
        .into_iter()
        .map(|p| ActivityProgressDto {
            kind: p.kind.as_str(),
            name: activity_name(p.kind, lang),
            level: p.state.level,
            xp: p.state.xp,
            xp_needed: p.xp_needed,
            overflow: p.state.overflow,
            cap: p.cap,
        })
        .collect();

    let catalog = ActivityKind::ALL
        .into_iter()
        .map(|kind| {
            let d = activity_def(kind);
            let min_stage = kind.min_stage();
            ActivityCatalogDto {
                kind: kind.as_str(),
                name: activity_name(kind, lang),
                min_stage: min_stage.as_str(),
                min_stage_name: stage_name(min_stage, lang),
                unlocked: ov.stage >= min_stage,
                base_duration_secs: (d.duration_secs * state.activity_duration_pct.max(1) / 100)
                    .max(1),
                xp_reward: d.xp_reward,
            }
        })
        .collect();

    Ok(Json(ActivitiesResponse {
        stage: ov.stage.as_str(),
        stage_name: stage_name(ov.stage, lang),
        slots_total: ov.slots_total,
        runs,
        progress,
        catalog,
        server_time: Utc::now().to_rfc3339(),
    }))
}

pub async fn start_activity(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
    Json(req): Json<StartActivityRequest>,
) -> Result<Json<ActivityRunDto>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let kind = ActivityKind::parse(req.kind.trim()).ok_or_else(|| {
        ApiError::bad_request("unknown_activity", format!("Unknown activity: {}", req.kind))
    })?;

    let voice_spd = state.voice_boost.speed_bonus_pct(auth.user_id);
    let run = activities::start_with_bonus(
        &state.pool,
        auth.user_id,
        kind,
        state.activity_duration_pct,
        voice_spd,
    )
    .await
        .map_err(|e| map_activity_error(e, lang))?;
    state.notify.notify_one();

    Ok(Json(ActivityRunDto {
        id: run.id,
        kind: run.kind.as_str(),
        name: activity_name(run.kind, lang),
        status: "running",
        ends_at: run.ends_at.to_rfc3339(),
    }))
}

pub async fn collect_activities(
    State(state): State<Arc<WebState>>,
    auth: AuthUser,
) -> Result<Json<CollectActivitiesResponse>, ApiError> {
    let lang = require_registered_lang(&state, auth.user_id).await?;
    let xp_bonus = state.voice_boost.xp_bonus_pct(auth.user_id);
    let summaries = activities::collect_ready_with_xp_bonus(&state.pool, auth.user_id, xp_bonus).await?;
    let collected = summaries
        .into_iter()
        .map(|s| CollectedRunDto {
            kind: s.kind.as_str(),
            name: activity_name(s.kind, lang),
            xp_gain: s.xp_gain,
            old_level: s.old_level,
            new_level: s.new_level,
            items: s
                .items
                .into_iter()
                .map(|(item, stored, pending)| CollectedItemDto {
                    item_id: item.id,
                    name: item_name(item.id, lang),
                    stored,
                    pending,
                })
                .collect(),
        })
        .collect();
    Ok(Json(CollectActivitiesResponse { collected }))
}

pub async fn api_not_found() -> ApiError {
    ApiError::not_found("not_found", "API endpoint not found")
}
