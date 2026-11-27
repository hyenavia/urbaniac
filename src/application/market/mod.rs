use crate::application::inventory;
use crate::domain::economy::{format_bac, npc_buy_price};
use crate::domain::i18n::{item_name, Lang};
use crate::domain::market::{
    calc_listing_fee, calc_sale_settlement, dynamic_price, next_multiplier, ListingDuration,
    MARKET_TICK_SECS, MAX_ACTIVE_LISTINGS, TICKS_PER_CALENDAR_DAY,
};
use crate::domain::resources::{find_item, ItemDef, ITEMS};
use crate::domain::world::{advance_calendar_day, world_price_multiplier, Season, WorldEvent};
use crate::infrastructure::repositories::wallet_repo::{self, ApplyOutcome, WalletError};
use crate::infrastructure::repositories::{inventory_repo, market_repo, player_repo, world_repo};
use crate::infrastructure::scheduler;
use chrono::{DateTime, Duration, Utc};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rust_decimal::Decimal;
use sqlx::{PgConnection, PgPool};
use std::collections::HashMap;

pub const JOB_BROKER_EXPIRE: &str = "broker_expire";
pub const JOB_MARKET_TICK: &str = "market_tick";

#[derive(Debug, thiserror::Error)]
pub enum MarketError {
    #[error("profil belum ada, jalankan /start")]
    NotRegistered,
    #[error("item tidak dikenal")]
    UnknownItem,
    #[error("jumlah tidak valid")]
    InvalidQty,
    #[error("harga target harus lebih dari 0")]
    InvalidPrice,
    #[error("maksimal {0} listing broker aktif")]
    TooManyListings(i64),
    #[error("jumlah item tidak cukup (kamu punya {have})")]
    InsufficientItem { have: i64 },
    #[error("saldo Bac tidak cukup untuk fee listing (butuh {need}, saldo {have})")]
    InsufficientBacForFee { need: Decimal, have: Decimal },
    #[error("listing tidak ditemukan atau sudah selesai")]
    ListingNotFound,
    #[error(transparent)]
    Wallet(#[from] WalletError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

pub struct MarketPriceView {
    pub item: &'static ItemDef,
    pub base_price: Decimal,
    pub current_price: Decimal,
    pub multiplier: Decimal,
}

pub struct BrokerListingView {
    pub id: i64,
    pub item: &'static ItemDef,
    pub quantity: i64,
    pub target_unit_price: Decimal,
    pub listing_fee: Decimal,
    pub expires_at: DateTime<Utc>,
}

pub struct CalendarView {
    pub day: i32,
    pub season: Season,
    pub active_event: Option<WorldEvent>,
    pub trend_item_id: Option<&'static str>,
    pub trend_item_name: Option<&'static str>,
}

pub struct MarketOverview {
    pub prices: Vec<MarketPriceView>,
    pub listings: Vec<BrokerListingView>,
    pub wallet_balance: Decimal,
    pub calendar: CalendarView,
}

fn to_calendar_view(row: &world_repo::CalendarRow) -> CalendarView {
    let season = Season::parse(&row.season).unwrap_or(Season::Spring);
    let active_event = row.active_event.as_deref().and_then(WorldEvent::parse);
    let trend_item = row.trend_item_id.as_deref().and_then(find_item);
    CalendarView {
        day: row.day,
        season,
        active_event,
        trend_item_id: trend_item.map(|i| i.id),
        trend_item_name: trend_item.map(|i| i.name),
    }
}

pub async fn current_npc_price_conn(
    conn: &mut PgConnection,
    item: &ItemDef,
) -> sqlx::Result<Decimal> {
    let mult = market_repo::get_multiplier(conn, item.id).await?;
    let cal = world_repo::get_calendar(conn).await?;
    let season = Season::parse(&cal.season).unwrap_or(Season::Spring);
    let event = cal.active_event.as_deref().and_then(WorldEvent::parse);
    let world_mult = world_price_multiplier(season, event, cal.trend_item_id.as_deref(), item.id);
    let combined = (mult * world_mult).round_dp(4);
    Ok(dynamic_price(npc_buy_price(item), combined))
}

pub async fn current_npc_price(pool: &PgPool, item: &ItemDef) -> anyhow::Result<Decimal> {
    let mut conn = pool.acquire().await?;
    Ok(current_npc_price_conn(&mut *conn, item).await?)
}

pub async fn overview(pool: &PgPool, player_id: i64) -> anyhow::Result<MarketOverview> {
    let mut conn = pool.acquire().await?;
    let cal_row = world_repo::get_calendar(&mut *conn).await?;
    let calendar = to_calendar_view(&cal_row);
    let rows = market_repo::list_prices(&mut *conn).await?;
    let prices = rows
        .into_iter()
        .filter_map(|r| {
            let item = find_item(&r.item_id)?;
            let base = npc_buy_price(item);
            let world_mult = world_price_multiplier(
                calendar.season,
                calendar.active_event,
                cal_row.trend_item_id.as_deref(),
                item.id,
            );
            let total_mult = (r.multiplier * world_mult).round_dp(4);
            let current = dynamic_price(base, total_mult);
            Some(MarketPriceView {
                item,
                base_price: base,
                current_price: current,
                multiplier: total_mult,
            })
        })
        .collect();

    let listings = market_repo::list_player_active(&mut *conn, player_id)
        .await?
        .into_iter()
        .filter_map(|l| {
            let item = find_item(&l.item_id)?;
            Some(BrokerListingView {
                id: l.id,
                item,
                quantity: l.quantity,
                target_unit_price: l.target_unit_price,
                listing_fee: l.listing_fee,
                expires_at: l.expires_at,
            })
        })
        .collect();

    let wallet_balance = wallet_repo::balance(&mut *conn, player_id)
        .await?
        .unwrap_or(Decimal::ZERO);

    Ok(MarketOverview {
        prices,
        listings,
        wallet_balance,
        calendar,
    })
}

/// Membuat listing di Makelar/Broker. Item dipindahkan dari inventory dan fee dipotong dari Wallet.
pub async fn create_listing(
    pool: &PgPool,
    player_id: i64,
    item_id: &str,
    qty: i64,
    target_unit_price: Decimal,
    duration: ListingDuration,
    duration_pct: i64,
    idempotency_key: &str,
) -> Result<BrokerListingView, MarketError> {
    let item = find_item(item_id).ok_or(MarketError::UnknownItem)?;
    if qty <= 0 {
        return Err(MarketError::InvalidQty);
    }
    let target_unit_price = target_unit_price.round_dp(2);
    if target_unit_price <= Decimal::ZERO {
        return Err(MarketError::InvalidPrice);
    }

    let mut tx = pool.begin().await?;
    if !player_repo::lock(&mut *tx, player_id).await? {
        return Err(MarketError::NotRegistered);
    }

    let active_count = market_repo::count_active_listings(&mut *tx, player_id).await?;
    if active_count >= MAX_ACTIVE_LISTINGS {
        return Err(MarketError::TooManyListings(MAX_ACTIVE_LISTINGS));
    }

    let have = inventory_repo::quantity_of(&mut *tx, player_id, item.id).await?;
    if have < qty {
        return Err(MarketError::InsufficientItem { have });
    }

    let fee = calc_listing_fee(target_unit_price, qty);
    let balance = wallet_repo::balance(&mut *tx, player_id)
        .await?
        .unwrap_or(Decimal::ZERO);
    if balance < fee {
        return Err(MarketError::InsufficientBacForFee {
            need: fee,
            have: balance,
        });
    }

    let reason = format!("broker_fee:{}x{}", item.id, qty);
    match wallet_repo::apply(&mut *tx, player_id, -fee, &reason, idempotency_key).await? {
        ApplyOutcome::Duplicate => return Err(MarketError::ListingNotFound),
        ApplyOutcome::Applied { .. } => {}
    }

    inventory_repo::remove_item(&mut *tx, player_id, item.id, qty).await?;

    let secs = (duration.secs() * duration_pct.max(1) / 100).max(1);
    let (id, expires_at) = market_repo::insert_listing(
        &mut *tx,
        player_id,
        item.id,
        qty,
        target_unit_price,
        fee,
        secs,
    )
    .await?;
    scheduler::enqueue(&mut *tx, JOB_BROKER_EXPIRE, id, expires_at).await?;
    tx.commit().await?;

    Ok(BrokerListingView {
        id,
        item,
        quantity: qty,
        target_unit_price,
        listing_fee: fee,
        expires_at,
    })
}

/// Membatalkan listing aktif milik pemain dan mengembalikan item ke inventory / Pending Claim.
pub async fn cancel_listing(
    pool: &PgPool,
    player_id: i64,
    listing_id: i64,
) -> Result<( &'static ItemDef, i64, i64 ), MarketError> {
    let mut tx = pool.begin().await?;
    if !player_repo::lock(&mut *tx, player_id).await? {
        return Err(MarketError::NotRegistered);
    }
    let Some(listing) = market_repo::lock_listing(&mut *tx, listing_id).await? else {
        return Err(MarketError::ListingNotFound);
    };
    if listing.player_id != player_id || listing.status != "active" {
        return Err(MarketError::ListingNotFound);
    }
    let item = find_item(&listing.item_id).ok_or(MarketError::UnknownItem)?;
    let cap = inventory::slot_capacity_conn(&mut *tx, player_id).await?;
    let placed = inventory_repo::add_item(&mut *tx, player_id, item, listing.quantity, cap).await?;
    market_repo::mark_closed(&mut *tx, listing_id, "cancelled").await?;
    tx.commit().await?;
    Ok((item, placed.stored, placed.pending))
}

/// Dipanggil worker saat durasi listing habis. Idempoten: hanya memproses listing 'active'.
pub async fn expire_listing(
    conn: &mut PgConnection,
    listing_id: i64,
) -> anyhow::Result<Option<(i64, String)>> {
    let Some(listing) = market_repo::lock_listing(conn, listing_id).await? else {
        return Ok(None);
    };
    if listing.status != "active" {
        return Ok(None);
    }
    let lang = player_repo::get_lang(conn, listing.player_id)
        .await
        .unwrap_or(Lang::En);
    let mut note = match lang {
        Lang::En => format!("📦 Broker Listing #{} expired.", listing.id),
        Lang::Id => format!("📦 Listing Makelar #{} kedaluwarsa.", listing.id),
    };
    if let Some(item) = find_item(&listing.item_id) {
        let cap = inventory::slot_capacity_conn(conn, listing.player_id).await?;
        let p = inventory_repo::add_item(conn, listing.player_id, item, listing.quantity, cap)
            .await?;
        let it_name = item_name(item.id, lang);
        note = if p.pending > 0 {
            match lang {
                Lang::En => format!(
                    "📦 Broker Listing **{} × {}** expired. ⚠️ Inventory full — moved to **Pending Claim**!",
                    it_name, listing.quantity
                ),
                Lang::Id => format!(
                    "📦 Listing Makelar **{} × {}** kedaluwarsa. ⚠️ Inventory penuh — masuk **Pending Claim**!",
                    it_name, listing.quantity
                ),
            }
        } else {
            match lang {
                Lang::En => format!(
                    "📦 Broker Listing **{} × {}** expired and was returned to your inventory.",
                    it_name, listing.quantity
                ),
                Lang::Id => format!(
                    "📦 Listing Makelar **{} × {}** kedaluwarsa dan dikembalikan ke inventory.",
                    it_name, listing.quantity
                ),
            }
        };
    }
    market_repo::mark_closed(conn, listing_id, "expired").await?;
    Ok(Some((listing.player_id, note)))
}

/// Memperbarui harga dinamis NPC dan menyelesaikan listing broker yang target harganya tercapai.
/// Dapat dipanggil dari scheduler (`market_tick`) maupun test.
pub async fn run_market_tick_conn(
    conn: &mut PgConnection,
    fixed_demand_bps: Option<i64>,
) -> anyhow::Result<usize> {
    let notes = run_market_tick_with_notes(conn, fixed_demand_bps, fixed_demand_bps.is_none()).await?;
    Ok(notes.len())
}

async fn run_market_tick_with_notes(
    conn: &mut PgConnection,
    fixed_demand_bps: Option<i64>,
    advance_day: bool,
) -> anyhow::Result<Vec<(i64, String)>> {
    let mut rng = StdRng::from_entropy();
    let cal_row = world_repo::get_calendar(conn).await?;
    let (mut season, mut event, mut trend_id) = (
        Season::parse(&cal_row.season).unwrap_or(Season::Spring),
        cal_row.active_event.as_deref().and_then(WorldEvent::parse),
        cal_row.trend_item_id.clone(),
    );

    if fixed_demand_bps.is_none() && advance_day {
        let (new_day, new_season, season_changed) = advance_calendar_day(cal_row.day, season);
        season = new_season;
        if season_changed || rng.gen_range(0..100) < 30 {
            event = if rng.gen_bool(0.6) {
                Some(WorldEvent::ALL[rng.gen_range(0..WorldEvent::ALL.len())])
            } else {
                None
            };
            trend_id = Some(ITEMS[rng.gen_range(0..ITEMS.len())].id.to_string());
        }
        world_repo::save_calendar(
            conn,
            new_day,
            season.as_str(),
            event.map(|e| e.as_str()),
            trend_id.as_deref(),
        )
        .await?;
    }

    let rows = market_repo::list_prices(conn).await?;
    let mut current_prices: HashMap<String, Decimal> = HashMap::new();

    for r in rows {
        let Some(item) = find_item(&r.item_id) else {
            continue;
        };
        let step_bps = fixed_demand_bps.unwrap_or_else(|| rng.gen_range(-800..=800));
        let new_mult = next_multiplier(r.multiplier, r.recent_sales, step_bps);
        market_repo::update_price(conn, &r.item_id, new_mult).await?;
        let world_mult = world_price_multiplier(season, event, trend_id.as_deref(), item.id);
        let combined = (new_mult * world_mult).round_dp(4);
        current_prices.insert(r.item_id, dynamic_price(npc_buy_price(item), combined));
    }

    let active = market_repo::list_all_active(conn).await?;
    let mut notes = Vec::new();
    for l in active {
        let Some(&mkt_price) = current_prices.get(&l.item_id) else {
            continue;
        };
        if mkt_price < l.target_unit_price {
            continue;
        }
        let Some(locked) = market_repo::lock_listing(conn, l.id).await? else {
            continue;
        };
        if locked.status != "active" {
            continue;
        }
        let (_, commission, net) = calc_sale_settlement(mkt_price, locked.quantity);
        let reason = format!(
            "broker_sale:{}x{}@{}(comm:{})",
            locked.item_id, locked.quantity, mkt_price, commission
        );
        let key = format!("broker_sale:{}", locked.id);
        let _ = wallet_repo::apply(conn, locked.player_id, net, &reason, &key).await?;
        market_repo::mark_sold(conn, locked.id, mkt_price, commission).await?;
        let lang = player_repo::get_lang(conn, locked.player_id)
            .await
            .unwrap_or(Lang::En);
        let item_label = find_item(&locked.item_id)
            .map(|i| item_name(i.id, lang))
            .unwrap_or(&locked.item_id);
        let msg = match lang {
            Lang::En => format!(
                "💰 Broker Listing **{} × {}** sold! Net **{}** credited to your Wallet.",
                item_label,
                locked.quantity,
                format_bac(net)
            ),
            Lang::Id => format!(
                "💰 Listing Makelar **{} × {}** terjual! Bersih **{}** masuk ke Wallet.",
                item_label,
                locked.quantity,
                format_bac(net)
            ),
        };
        notes.push((locked.player_id, msg));
    }

    Ok(notes)
}

/// Dipanggil oleh scheduler worker untuk `market_tick`. Menjadwalkan tick berikutnya.
pub async fn handle_market_tick_job(
    conn: &mut PgConnection,
    tick_ref: i64,
    duration_pct: i64,
) -> anyhow::Result<Vec<(i64, String)>> {
    let advance_day = tick_ref % TICKS_PER_CALENDAR_DAY == 0;
    let notes = run_market_tick_with_notes(conn, None, advance_day).await?;
    let secs = (MARKET_TICK_SECS * duration_pct.max(1) / 100).max(5);
    let next_ref = market_repo::next_market_tick_ref(conn).await?;
    scheduler::enqueue(conn, JOB_MARKET_TICK, next_ref, Utc::now() + Duration::seconds(secs))
        .await?;
    Ok(notes)
}

/// Memastikan ada minimal satu job `market_tick` pending saat bot mulai.
pub async fn ensure_market_tick_scheduled(pool: &PgPool, duration_pct: i64) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;
    if !market_repo::has_pending_market_tick(&mut *conn).await? {
        let secs = (MARKET_TICK_SECS * duration_pct.max(1) / 100).max(5);
        let next_ref = market_repo::next_market_tick_ref(&mut *conn).await?;
        scheduler::enqueue(
            &mut *conn,
            JOB_MARKET_TICK,
            next_ref,
            Utc::now() + Duration::seconds(secs),
        )
        .await?;
    }
    Ok(())
}

pub async fn advance_world_day(
    pool: &PgPool,
    new_season: Option<Season>,
    event: Option<WorldEvent>,
    trend_item: Option<&str>,
) -> anyhow::Result<CalendarView> {
    let mut tx = pool.begin().await?;
    let cur = world_repo::get_calendar(&mut *tx).await?;
    let (next_day, auto_season, _) =
        advance_calendar_day(cur.day, Season::parse(&cur.season).unwrap_or(Season::Spring));
    let season = new_season.unwrap_or(auto_season);
    world_repo::save_calendar(
        &mut *tx,
        next_day,
        season.as_str(),
        event.map(|e| e.as_str()),
        trend_item,
    )
    .await?;
    let updated = world_repo::get_calendar(&mut *tx).await?;
    tx.commit().await?;
    Ok(to_calendar_view(&updated))
}
