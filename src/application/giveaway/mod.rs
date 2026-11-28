use crate::application::inventory;
use crate::domain::economy::format_bac;
use crate::domain::resources::find_item;
use crate::infrastructure::repositories::giveaway_repo::{self, Giveaway};
use crate::infrastructure::repositories::wallet_repo::{self, ApplyOutcome, WalletError};
use crate::infrastructure::repositories::{inventory_repo, player_repo};
use crate::infrastructure::scheduler;
use anyhow::bail;
use chrono::{Duration, Utc};
use rand::seq::SliceRandom;
use rust_decimal::Decimal;
use sqlx::{PgConnection, PgPool};
use tokio::sync::Notify;

pub const JOB_GIVEAWAY_END: &str = "giveaway_end";
pub const KIND_CASH: &str = "cash";
pub const KIND_ITEM: &str = "item";

pub const MIN_DURATION_MINUTES: i64 = 1;
pub const MAX_DURATION_MINUTES: i64 = 43_200; // 30 days
pub const MIN_WINNERS: i32 = 1;
pub const MAX_WINNERS: i32 = 20;
pub const MAX_CASH_AMOUNT: i64 = 1_000_000_000;
pub const MAX_ITEM_QTY: i64 = 1_000_000;

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

pub fn validate_cash_amount(amount: i64) -> anyhow::Result<Decimal> {
    if !(1..=MAX_CASH_AMOUNT).contains(&amount) {
        bail!("Cash amount must be between 1 and {MAX_CASH_AMOUNT} Bac.");
    }
    Ok(Decimal::from(amount).round_dp(2))
}

pub fn validate_item_qty(qty: i64) -> anyhow::Result<i64> {
    if !(1..=MAX_ITEM_QTY).contains(&qty) {
        bail!("Item quantity must be between 1 and {MAX_ITEM_QTY}.");
    }
    Ok(qty)
}

pub fn format_cash_prize(amount: Decimal, note: Option<&str>) -> String {
    let base = format_bac(amount);
    match note.map(str::trim).filter(|s| !s.is_empty()) {
        Some(n) => format!("{base} — {n}"),
        None => base,
    }
}

pub fn format_item_prize(item_name: &str, qty: i64, note: Option<&str>) -> String {
    let base = format!("{qty}x {item_name}");
    match note.map(str::trim).filter(|s| !s.is_empty()) {
        Some(n) => format!("{base} — {n}"),
        None => base,
    }
}

#[allow(clippy::too_many_arguments)]
async fn create_scheduled_giveaway(
    pool: &PgPool,
    notify: &Notify,
    guild_id: u64,
    channel_id: u64,
    host_id: u64,
    kind: &str,
    prize: &str,
    cash_amount: Decimal,
    item_id: Option<&str>,
    item_qty: i64,
    winner_count: i32,
    duration_minutes: i64,
) -> anyhow::Result<Giveaway> {
    let ends_at = Utc::now() + Duration::minutes(duration_minutes);
    let mut tx = pool.begin().await?;
    let gw = giveaway_repo::create_giveaway(
        &mut *tx,
        guild_id as i64,
        channel_id as i64,
        host_id as i64,
        kind,
        prize,
        cash_amount,
        item_id,
        item_qty,
        winner_count,
        ends_at,
    )
    .await?;
    scheduler::enqueue(&mut *tx, JOB_GIVEAWAY_END, gw.id, gw.ends_at).await?;
    tx.commit().await?;
    notify.notify_one();
    Ok(gw)
}

#[allow(clippy::too_many_arguments)]
pub async fn start_cash_giveaway(
    pool: &PgPool,
    notify: &Notify,
    guild_id: u64,
    channel_id: u64,
    host_id: u64,
    amount: i64,
    winner_count: i32,
    duration_minutes: i64,
    note: Option<&str>,
) -> anyhow::Result<Giveaway> {
    let cash_amount = validate_cash_amount(amount)?;
    validate_winner_count(winner_count)?;
    validate_duration_minutes(duration_minutes)?;
    let prize = format_cash_prize(cash_amount, note);

    create_scheduled_giveaway(
        pool,
        notify,
        guild_id,
        channel_id,
        host_id,
        KIND_CASH,
        &prize,
        cash_amount,
        None,
        0,
        winner_count,
        duration_minutes,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn start_item_giveaway(
    pool: &PgPool,
    notify: &Notify,
    guild_id: u64,
    channel_id: u64,
    host_id: u64,
    item_id: &str,
    qty: i64,
    winner_count: i32,
    duration_minutes: i64,
    note: Option<&str>,
) -> anyhow::Result<Giveaway> {
    let Some(item) = find_item(item_id) else {
        bail!("Unknown item `{item_id}`.");
    };
    let qty = validate_item_qty(qty)?;
    validate_winner_count(winner_count)?;
    validate_duration_minutes(duration_minutes)?;
    let prize = format_item_prize(item.name, qty, note);

    create_scheduled_giveaway(
        pool,
        notify,
        guild_id,
        channel_id,
        host_id,
        KIND_ITEM,
        &prize,
        Decimal::ZERO,
        Some(item.id),
        qty,
        winner_count,
        duration_minutes,
    )
    .await
}

pub async fn attach_message_id(
    pool: &PgPool,
    giveaway_id: i64,
    message_id: u64,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;
    giveaway_repo::set_message_id(&mut *conn, giveaway_id, message_id as i64).await?;
    Ok(())
}

pub async fn rollback_unposted(pool: &PgPool, giveaway_id: i64) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    giveaway_repo::delete_unposted(&mut *tx, giveaway_id).await?;
    tx.commit().await?;
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
    let Some(gw) = giveaway_repo::get_giveaway(&mut *conn, giveaway_id).await? else {
        return Ok(EnterOutcome::NotFound);
    };
    if gw.status != "active" || gw.ends_at <= Utc::now() {
        return Ok(EnterOutcome::Ended);
    }
    if player_repo::get(&mut *conn, user_id).await?.is_none() {
        return Ok(EnterOutcome::NeedsProfile);
    }
    let inserted = giveaway_repo::add_entry(&mut *conn, giveaway_id, user_id).await?;
    let total_entries = giveaway_repo::count_entries(&mut *conn, giveaway_id).await?;
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
    pub pending_winners: Vec<i64>,
    pub skipped_no_profile: Vec<i64>,
    pub total_entries: i64,
    pub is_reroll: bool,
}

enum CreditStatus {
    Credited { went_pending: bool },
    SkippedNoProfile,
}

async fn credit_winner(
    conn: &mut PgConnection,
    gw: &Giveaway,
    winner_id: i64,
    idempotency_key: &str,
) -> anyhow::Result<CreditStatus> {
    if gw.kind == KIND_CASH && gw.cash_amount > Decimal::ZERO {
        let reason = format!("giveaway:{}", gw.id);
        return match wallet_repo::apply(conn, winner_id, gw.cash_amount, &reason, idempotency_key).await {
            Ok(ApplyOutcome::Applied { .. }) | Ok(ApplyOutcome::Duplicate) => {
                Ok(CreditStatus::Credited { went_pending: false })
            }
            Err(WalletError::NotFound) => Ok(CreditStatus::SkippedNoProfile),
            Err(e) => Err(e.into()),
        };
    }

    if gw.kind == KIND_ITEM && gw.item_qty > 0 {
        let Some(item_id) = gw.item_id.as_deref() else {
            bail!("Giveaway #{} is missing item_id", gw.id);
        };
        let Some(item) = find_item(item_id) else {
            bail!("Giveaway #{} references unknown item `{item_id}`", gw.id);
        };
        if !player_repo::lock(conn, winner_id).await? {
            return Ok(CreditStatus::SkippedNoProfile);
        }
        let cap = inventory::slot_capacity_conn(conn, winner_id).await?;
        let placement = inventory_repo::add_item(conn, winner_id, item, gw.item_qty, cap).await?;
        return Ok(CreditStatus::Credited {
            went_pending: placement.pending > 0,
        });
    }

    Ok(CreditStatus::Credited { went_pending: false })
}

async fn distribute_rewards(
    conn: &mut PgConnection,
    gw: &Giveaway,
    winners: &[i64],
    reroll_interaction_id: Option<u64>,
) -> anyhow::Result<(Vec<i64>, Vec<i64>, Vec<i64>)> {
    let mut credited = Vec::new();
    let mut pending = Vec::new();
    let mut skipped = Vec::new();

    for &winner_id in winners {
        let key = match reroll_interaction_id {
            Some(iid) => format!("giveaway:{}:reroll:{iid}:{winner_id}", gw.id),
            None => format!("giveaway:{}:{winner_id}", gw.id),
        };
        match credit_winner(conn, gw, winner_id, &key).await? {
            CreditStatus::Credited { went_pending } => {
                credited.push(winner_id);
                if went_pending {
                    pending.push(winner_id);
                }
            }
            CreditStatus::SkippedNoProfile => skipped.push(winner_id),
        }
    }

    Ok((credited, pending, skipped))
}

async fn finalize_locked_giveaway(
    conn: &mut PgConnection,
    mut gw: Giveaway,
) -> anyhow::Result<Option<GiveawayFinishResult>> {
    if gw.status != "active" {
        return Ok(None);
    }

    let entries = giveaway_repo::list_entries(conn, gw.id).await?;
    let total_entries = entries.len() as i64;
    let winners = pick_winners(&entries, gw.winner_count.max(0) as usize);
    let (credited_winners, pending_winners, skipped_no_profile) =
        distribute_rewards(conn, &gw, &winners, None).await?;

    giveaway_repo::mark_ended(conn, gw.id, &winners).await?;
    gw.status = "ended".to_string();
    gw.winner_ids = winners.clone();

    Ok(Some(GiveawayFinishResult {
        giveaway: gw,
        winners,
        credited_winners,
        pending_winners,
        skipped_no_profile,
        total_entries,
        is_reroll: false,
    }))
}

/// Finalizes an active giveaway inside an existing transaction (used by scheduler).
pub async fn finalize_giveaway_conn(
    conn: &mut PgConnection,
    giveaway_id: i64,
) -> anyhow::Result<Option<GiveawayFinishResult>> {
    let Some(gw) = giveaway_repo::lock_giveaway(conn, giveaway_id).await? else {
        return Ok(None);
    };
    finalize_locked_giveaway(conn, gw).await
}

pub async fn end_giveaway_now(
    pool: &PgPool,
    guild_id: u64,
    giveaway_id: i64,
) -> anyhow::Result<Option<GiveawayFinishResult>> {
    let mut tx = pool.begin().await?;
    let Some(gw) = giveaway_repo::lock_giveaway(&mut *tx, giveaway_id).await? else {
        return Ok(None);
    };
    if gw.guild_id != guild_id as i64 {
        return Ok(None);
    }
    let res = finalize_locked_giveaway(&mut *tx, gw).await?;
    tx.commit().await?;
    Ok(res)
}

pub async fn reroll_giveaway(
    pool: &PgPool,
    guild_id: u64,
    giveaway_id: i64,
    interaction_id: u64,
) -> anyhow::Result<Option<GiveawayFinishResult>> {
    let mut tx = pool.begin().await?;
    let Some(mut gw) = giveaway_repo::lock_giveaway(&mut *tx, giveaway_id).await? else {
        return Ok(None);
    };
    if gw.guild_id != guild_id as i64 || gw.status != "ended" {
        return Ok(None);
    }

    let entries = giveaway_repo::list_entries(&mut *tx, gw.id).await?;
    let total_entries = entries.len() as i64;
    let remaining: Vec<i64> = entries
        .iter()
        .copied()
        .filter(|id| !gw.winner_ids.contains(id))
        .collect();
    let candidate_pool = if remaining.is_empty() { &entries } else { &remaining };
    let winners = pick_winners(candidate_pool, 1);

    let (credited_winners, pending_winners, skipped_no_profile) =
        distribute_rewards(&mut *tx, &gw, &winners, Some(interaction_id)).await?;

    if let Some(&winner_id) = winners.first() {
        if !gw.winner_ids.contains(&winner_id) {
            gw.winner_ids.push(winner_id);
            giveaway_repo::update_winners(&mut *tx, gw.id, &gw.winner_ids).await?;
        }
    }

    tx.commit().await?;
    Ok(Some(GiveawayFinishResult {
        giveaway: gw,
        winners,
        credited_winners,
        pending_winners,
        skipped_no_profile,
        total_entries,
        is_reroll: true,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::resources::ITEMS;
    use std::collections::HashSet;

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

        assert!(validate_cash_amount(0).is_err());
        assert!(validate_cash_amount(MAX_CASH_AMOUNT + 1).is_err());
        assert_eq!(validate_cash_amount(500).unwrap(), Decimal::new(50_000, 2));

        assert!(validate_item_qty(0).is_err());
        assert!(validate_item_qty(MAX_ITEM_QTY + 1).is_err());
        assert_eq!(validate_item_qty(25).unwrap(), 25);
    }

    #[test]
    fn formats_prize_strings() {
        assert_eq!(
            format_cash_prize(Decimal::new(50_000, 2), None),
            "500.00 Bac"
        );
        assert_eq!(
            format_cash_prize(Decimal::new(50_000, 2), Some("  Weekend Event  ")),
            "500.00 Bac — Weekend Event"
        );
        assert_eq!(format_item_prize("Star Crystal", 5, None), "5x Star Crystal");
        assert_eq!(
            format_item_prize("Star Crystal", 5, Some("Launch Bonus")),
            "5x Star Crystal — Launch Bonus"
        );
    }

    #[test]
    fn item_catalog_fits_discord_choice_limit() {
        assert!(ITEMS.len() <= 25);
    }
}
