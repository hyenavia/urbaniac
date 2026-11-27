use crate::infrastructure::repositories::server_repo::{self, Giveaway, WelcomeConfig};
use crate::infrastructure::repositories::wallet_repo::{self, ApplyOutcome, WalletError};
use crate::infrastructure::repositories::player_repo;
use crate::infrastructure::scheduler;
use anyhow::bail;
use chrono::{Duration, Utc};
use rand::seq::SliceRandom;
use rust_decimal::Decimal;
use sqlx::{PgConnection, PgPool};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};

pub const JOB_GIVEAWAY_END: &str = "giveaway_end";
pub const KIND_STANDARD: &str = "standard";
pub const KIND_CASH: &str = "cash";

pub const MIN_DURATION_MINUTES: i64 = 1;
pub const MAX_DURATION_MINUTES: i64 = 43_200; // 30 days
pub const MIN_WINNERS: i32 = 1;
pub const MAX_WINNERS: i32 = 20;

pub type WelcomeCache = Arc<RwLock<HashMap<u64, WelcomeConfig>>>;

/// Replaces supported welcome placeholders in a title or message template.
pub fn render_welcome_template(
    template: &str,
    user_id: u64,
    username: &str,
    server_name: &str,
    member_count: Option<u64>,
) -> String {
    let mention = format!("<@{user_id}>");
    let count = member_count
        .map(|n| n.to_string())
        .unwrap_or_else(|| "?".to_string());
    template
        .replace("{user}", &mention)
        .replace("{username}", username)
        .replace("{server}", server_name)
        .replace("{member_count}", &count)
}

/// Returns true if the welcome template references guild metadata that requires an API lookup.
pub fn needs_guild_lookup(title: &str, message: &str) -> bool {
    title.contains("{server}")
        || title.contains("{member_count}")
        || message.contains("{server}")
        || message.contains("{member_count}")
}

/// Picks up to `count` unique random winners from `entries`.
pub fn pick_winners(entries: &[i64], count: usize) -> Vec<i64> {
    if entries.is_empty() || count == 0 {
        return Vec::new();
    }
    let mut rng = rand::thread_rng();
    entries
        .choose_multiple(&mut rng, count.min(entries.len()))
        .copied()
        .collect()
}

pub fn validate_duration_minutes(minutes: i64) -> anyhow::Result<i64> {
    if !(MIN_DURATION_MINUTES..=MAX_DURATION_MINUTES).contains(&minutes) {
        bail!("Duration must be between {MIN_DURATION_MINUTES} and {MAX_DURATION_MINUTES} minutes.");
    }
    Ok(minutes)
}

pub fn validate_winner_count(winners: i32) -> anyhow::Result<i32> {
    if !(MIN_WINNERS..=MAX_WINNERS).contains(&winners) {
        bail!("Winner count must be between {MIN_WINNERS} and {MAX_WINNERS}.");
    }
    Ok(winners)
}

pub async fn load_welcome_cache(pool: &PgPool) -> anyhow::Result<WelcomeCache> {
    let mut conn = pool.acquire().await?;
    let rows = server_repo::list_all_welcomes(&mut *conn).await?;
    let map = rows
        .into_iter()
        .map(|cfg| (cfg.guild_id as u64, cfg))
        .collect();
    Ok(Arc::new(RwLock::new(map)))
}

pub async fn save_welcome(
    pool: &PgPool,
    cache: &WelcomeCache,
    cfg: WelcomeConfig,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;
    server_repo::upsert_welcome(&mut *conn, &cfg).await?;
    drop(conn);
    cache.write().await.insert(cfg.guild_id as u64, cfg);
    Ok(())
}

pub async fn disable_welcome(
    pool: &PgPool,
    cache: &WelcomeCache,
    guild_id: u64,
) -> anyhow::Result<bool> {
    let mut conn = pool.acquire().await?;
    let updated = server_repo::set_welcome_enabled(&mut *conn, guild_id as i64, false).await?;
    drop(conn);
    if updated {
        if let Some(cfg) = cache.write().await.get_mut(&guild_id) {
            cfg.enabled = false;
        }
    }
    Ok(updated)
}

pub async fn get_welcome_cached(cache: &WelcomeCache, guild_id: u64) -> Option<WelcomeConfig> {
    cache.read().await.get(&guild_id).cloned()
}

#[allow(clippy::too_many_arguments)]
pub async fn start_giveaway(
    pool: &PgPool,
    notify: &Notify,
    guild_id: u64,
    channel_id: u64,
    host_id: u64,
    kind: &str,
    prize: &str,
    cash_amount: Decimal,
    winner_count: i32,
    duration_minutes: i64,
) -> anyhow::Result<Giveaway> {
    validate_duration_minutes(duration_minutes)?;
    validate_winner_count(winner_count)?;
    if kind == KIND_CASH && cash_amount <= Decimal::ZERO {
        bail!("Cash reward amount must be greater than 0.");
    }
    let ends_at = Utc::now() + Duration::minutes(duration_minutes);

    let mut tx = pool.begin().await?;
    let gw = server_repo::create_giveaway(
        &mut *tx,
        guild_id as i64,
        channel_id as i64,
        host_id as i64,
        kind,
        prize,
        cash_amount.round_dp(2),
        winner_count,
        ends_at,
    )
    .await?;
    scheduler::enqueue(&mut *tx, JOB_GIVEAWAY_END, gw.id, gw.ends_at).await?;
    tx.commit().await?;
    notify.notify_one();
    Ok(gw)
}

pub async fn attach_message_id(
    pool: &PgPool,
    giveaway_id: i64,
    message_id: u64,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;
    server_repo::set_giveaway_message_id(&mut *conn, giveaway_id, message_id as i64).await?;
    Ok(())
}

#[derive(Debug)]
pub enum EnterOutcome {
    Entered { giveaway: Giveaway, total_entries: i64 },
    AlreadyEntered { total_entries: i64 },
    NotFound,
    Ended,
    NeedsProfile,
}

pub async fn enter_giveaway(
    pool: &PgPool,
    giveaway_id: i64,
    user_id: i64,
) -> anyhow::Result<EnterOutcome> {
    let mut conn = pool.acquire().await?;
    let Some(gw) = server_repo::get_giveaway(&mut *conn, giveaway_id).await? else {
        return Ok(EnterOutcome::NotFound);
    };
    if gw.status != "active" || gw.ends_at <= Utc::now() {
        return Ok(EnterOutcome::Ended);
    }
    if gw.kind == KIND_CASH && player_repo::get(&mut *conn, user_id).await?.is_none() {
        return Ok(EnterOutcome::NeedsProfile);
    }
    let inserted = server_repo::add_entry(&mut *conn, giveaway_id, user_id).await?;
    let total_entries = server_repo::count_entries(&mut *conn, giveaway_id).await?;
    if inserted {
        Ok(EnterOutcome::Entered { giveaway: gw, total_entries })
    } else {
        Ok(EnterOutcome::AlreadyEntered { total_entries })
    }
}

#[derive(Debug, Clone)]
pub struct GiveawayFinishResult {
    pub giveaway: Giveaway,
    pub winners: Vec<i64>,
    pub credited_winners: Vec<i64>,
    pub skipped_no_wallet: Vec<i64>,
    pub total_entries: i64,
    pub is_reroll: bool,
}

async fn credit_cash_winner(
    conn: &mut PgConnection,
    giveaway_id: i64,
    winner_id: i64,
    cash_amount: Decimal,
    idempotency_key: &str,
) -> anyhow::Result<bool> {
    let reason = format!("giveaway:{giveaway_id}");
    match wallet_repo::apply(conn, winner_id, cash_amount, &reason, idempotency_key).await {
        Ok(ApplyOutcome::Applied { .. }) | Ok(ApplyOutcome::Duplicate) => Ok(true),
        Err(WalletError::NotFound) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Finalizes an active giveaway inside an existing transaction (used by scheduler and `/giveaway end`).
pub async fn finalize_giveaway_conn(
    conn: &mut PgConnection,
    giveaway_id: i64,
) -> anyhow::Result<Option<GiveawayFinishResult>> {
    let Some(mut gw) = server_repo::lock_giveaway(&mut *conn, giveaway_id).await? else {
        return Ok(None);
    };
    if gw.status != "active" {
        return Ok(None);
    }

    let entries = server_repo::list_entries(&mut *conn, gw.id).await?;
    let total_entries = entries.len() as i64;
    let winners = pick_winners(&entries, gw.winner_count.max(0) as usize);

    let mut credited_winners = Vec::new();
    let mut skipped_no_wallet = Vec::new();
    if gw.kind == KIND_CASH && gw.cash_amount > Decimal::ZERO {
        for &winner_id in &winners {
            let key = format!("giveaway:{}:{}", gw.id, winner_id);
            if credit_cash_winner(&mut *conn, gw.id, winner_id, gw.cash_amount, &key).await? {
                credited_winners.push(winner_id);
            } else {
                skipped_no_wallet.push(winner_id);
            }
        }
    }

    server_repo::mark_giveaway_ended(&mut *conn, gw.id, &winners).await?;
    gw.status = "ended".to_string();
    gw.winner_ids = winners.clone();

    Ok(Some(GiveawayFinishResult {
        giveaway: gw,
        winners,
        credited_winners,
        skipped_no_wallet,
        total_entries,
        is_reroll: false,
    }))
}

pub async fn end_giveaway_now(
    pool: &PgPool,
    giveaway_id: i64,
) -> anyhow::Result<Option<GiveawayFinishResult>> {
    let mut tx = pool.begin().await?;
    let res = finalize_giveaway_conn(&mut *tx, giveaway_id).await?;
    tx.commit().await?;
    Ok(res)
}

pub async fn reroll_giveaway(
    pool: &PgPool,
    giveaway_id: i64,
    interaction_id: u64,
) -> anyhow::Result<Option<GiveawayFinishResult>> {
    let mut tx = pool.begin().await?;
    let Some(mut gw) = server_repo::lock_giveaway(&mut *tx, giveaway_id).await? else {
        return Ok(None);
    };
    if gw.status != "ended" {
        return Ok(None);
    }

    let entries = server_repo::list_entries(&mut *tx, gw.id).await?;
    let total_entries = entries.len() as i64;
    let remaining: Vec<i64> = entries
        .iter()
        .copied()
        .filter(|id| !gw.winner_ids.contains(id))
        .collect();
    let candidate_pool = if remaining.is_empty() { &entries } else { &remaining };
    let winners = pick_winners(candidate_pool, 1);

    let mut credited_winners = Vec::new();
    let mut skipped_no_wallet = Vec::new();
    if let Some(&winner_id) = winners.first() {
        if gw.kind == KIND_CASH && gw.cash_amount > Decimal::ZERO {
            let key = format!("giveaway:{}:reroll:{}:{}", gw.id, interaction_id, winner_id);
            if credit_cash_winner(&mut *tx, gw.id, winner_id, gw.cash_amount, &key).await? {
                credited_winners.push(winner_id);
            } else {
                skipped_no_wallet.push(winner_id);
            }
        }
        if !gw.winner_ids.contains(&winner_id) {
            gw.winner_ids.push(winner_id);
            server_repo::update_giveaway_winners(&mut *tx, gw.id, &gw.winner_ids).await?;
        }
    }

    tx.commit().await?;
    Ok(Some(GiveawayFinishResult {
        giveaway: gw,
        winners,
        credited_winners,
        skipped_no_wallet,
        total_entries,
        is_reroll: true,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn renders_welcome_placeholders() {
        let out = render_welcome_template(
            "Hello {user} ({username}) in {server} #{member_count}!",
            42,
            "Alice",
            "Urbaniac HQ",
            Some(128),
        );
        assert_eq!(out, "Hello <@42> (Alice) in Urbaniac HQ #128!");

        let fallback = render_welcome_template("{server} - {member_count}", 1, "Bob", "City", None);
        assert_eq!(fallback, "City - ?");
    }

    #[test]
    fn detects_guild_lookup_placeholders() {
        assert!(!needs_guild_lookup("Welcome!", "Glad to have you, {user}!"));
        assert!(needs_guild_lookup("Welcome to {server}!", "Hi {user}"));
        assert!(needs_guild_lookup("Welcome!", "You are member #{member_count}"));
    }

    #[test]
    fn picks_unique_winners_bounded_by_entries() {
        assert!(pick_winners(&[], 3).is_empty());
        assert!(pick_winners(&[1, 2, 3], 0).is_empty());

        let all = pick_winners(&[10, 20, 30], 5);
        assert_eq!(all.len(), 3);
        let unique: HashSet<i64> = all.into_iter().collect();
        assert_eq!(unique.len(), 3);

        let two = pick_winners(&[1, 2, 3, 4, 5], 2);
        assert_eq!(two.len(), 2);
        assert_ne!(two[0], two[1]);
    }

    #[test]
    fn validates_giveaway_bounds() {
        assert!(validate_duration_minutes(0).is_err());
        assert!(validate_duration_minutes(MAX_DURATION_MINUTES + 1).is_err());
        assert_eq!(validate_duration_minutes(60).unwrap(), 60);

        assert!(validate_winner_count(0).is_err());
        assert!(validate_winner_count(MAX_WINNERS + 1).is_err());
        assert_eq!(validate_winner_count(3).unwrap(), 3);
    }
}
