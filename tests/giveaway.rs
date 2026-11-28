//! Integration test untuk fitur /giveaway (cash & item in-game).
//! Butuh PostgreSQL (DATABASE_URL, user boleh CREATE DATABASE).
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Notify;
use urbaniac::application::giveaway::{self, EnterOutcome, KIND_CASH, KIND_ITEM};
use urbaniac::application::{economy, inventory, player};
use urbaniac::domain::resources::{find_item, BASE_SLOT_CAPACITY, ITEMS};
use urbaniac::infrastructure::repositories::inventory_repo;
use urbaniac::workers::process_due_batch;

#[sqlx::test(migrations = "./migrations")]
async fn cash_giveaway_flow_and_scheduler_payout(pool: PgPool) {
    player::register(&pool, 1001, "winner_one", None).await.unwrap();
    let bal_before = economy::balance(&pool, 1001).await.unwrap();

    let notify = Arc::new(Notify::new());
    let gw = giveaway::start_cash_giveaway(
        &pool,
        &notify,
        777,
        888,
        999,
        250,
        1,
        15,
        Some("Bonus"),
    )
    .await
    .unwrap();
    assert_eq!(gw.kind, KIND_CASH);
    assert_eq!(gw.cash_amount, Decimal::new(25_000, 2));

    // User tanpa profil ditolak saat join.
    assert!(matches!(
        giveaway::enter_giveaway(&pool, gw.id, 2002).await.unwrap(),
        EnterOutcome::NeedsProfile
    ));

    // User terdaftar bisa join sekali; duplikat terdeteksi.
    assert!(matches!(
        giveaway::enter_giveaway(&pool, gw.id, 1001).await.unwrap(),
        EnterOutcome::Entered { total_entries: 1, .. }
    ));
    assert!(matches!(
        giveaway::enter_giveaway(&pool, gw.id, 1001).await.unwrap(),
        EnterOutcome::AlreadyEntered { total_entries: 1 }
    ));

    // Jatuh tempokan job scheduler dan proses batch.
    sqlx::query("UPDATE scheduled_jobs SET run_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    let processed = process_due_batch(&pool).await.unwrap();
    assert!(processed >= 1);

    let bal_after = economy::balance(&pool, 1001).await.unwrap();
    assert_eq!(bal_after, bal_before + Decimal::new(25_000, 2));

    let status: String = sqlx::query_scalar("SELECT status FROM giveaways WHERE id = $1")
        .bind(gw.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "ended");
}

#[sqlx::test(migrations = "./migrations")]
async fn item_giveaway_delivery_and_pending_when_full(pool: PgPool) {
    player::register(&pool, 3001, "full_bag", None).await.unwrap();
    player::register(&pool, 3002, "open_bag", None).await.unwrap();

    // Penuhi seluruh slot inventory pemain 3001 dengan item selain star_crystal.
    let mut tx = pool.begin().await.unwrap();
    for item in ITEMS.iter().filter(|i| i.id != "star_crystal").take(BASE_SLOT_CAPACITY as usize) {
        inventory_repo::add_item(&mut *tx, 3001, item, 1, BASE_SLOT_CAPACITY)
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();

    let notify = Arc::new(Notify::new());
    let gw = giveaway::start_item_giveaway(
        &pool,
        &notify,
        777,
        888,
        999,
        "star_crystal",
        3,
        2,
        10,
        None,
    )
    .await
    .unwrap();
    assert_eq!(gw.kind, KIND_ITEM);

    assert!(matches!(
        giveaway::enter_giveaway(&pool, gw.id, 3001).await.unwrap(),
        EnterOutcome::Entered { .. }
    ));
    assert!(matches!(
        giveaway::enter_giveaway(&pool, gw.id, 3002).await.unwrap(),
        EnterOutcome::Entered { .. }
    ));

    let res = giveaway::end_giveaway_now(&pool, 777, gw.id)
        .await
        .unwrap()
        .expect("giveaway should end");
    assert_eq!(res.credited_winners.len(), 2);
    assert!(res.pending_winners.contains(&3001));
    assert!(!res.pending_winners.contains(&3002));

    let v_full = inventory::view(&pool, 3001).await.unwrap();
    assert!(v_full
        .pending
        .iter()
        .any(|l| l.item.id == "star_crystal" && l.quantity == 3));

    let v_open = inventory::view(&pool, 3002).await.unwrap();
    assert!(v_open
        .lines
        .iter()
        .any(|l| l.item.id == "star_crystal" && l.quantity == 3));
}

#[sqlx::test(migrations = "./migrations")]
async fn reroll_and_guild_isolation(pool: PgPool) {
    player::register(&pool, 4001, "p1", None).await.unwrap();
    player::register(&pool, 4002, "p2", None).await.unwrap();

    let notify = Arc::new(Notify::new());
    let gw = giveaway::start_item_giveaway(
        &pool,
        &notify,
        555,
        888,
        999,
        "bread",
        5,
        1,
        10,
        None,
    )
    .await
    .unwrap();

    giveaway::enter_giveaway(&pool, gw.id, 4001).await.unwrap();
    giveaway::enter_giveaway(&pool, gw.id, 4002).await.unwrap();

    // Guild berbeda tidak boleh bisa mengakhiri atau reroll giveaway guild lain.
    assert!(giveaway::end_giveaway_now(&pool, 999, gw.id).await.unwrap().is_none());

    let ended = giveaway::end_giveaway_now(&pool, 555, gw.id)
        .await
        .unwrap()
        .expect("matching guild can end");
    assert_eq!(ended.winners.len(), 1);
    let first_winner = ended.winners[0];

    assert!(giveaway::reroll_giveaway(&pool, 999, gw.id, 100).await.unwrap().is_none());

    let reroll = giveaway::reroll_giveaway(&pool, 555, gw.id, 101)
        .await
        .unwrap()
        .expect("matching guild can reroll");
    assert!(reroll.is_reroll);
    assert_eq!(reroll.winners.len(), 1);
    let second_winner = reroll.winners[0];
    assert_ne!(first_winner, second_winner);

    let bread = find_item("bread").unwrap();
    let v = inventory::view(&pool, second_winner).await.unwrap();
    assert!(v.lines.iter().any(|l| l.item.id == bread.id && l.quantity == 5));
}
