//! Integration test tahap 2. Butuh PostgreSQL (DATABASE_URL, user boleh CREATE DATABASE).
use rust_decimal::Decimal;
use sqlx::PgPool;
use urbaniac::application::activities::{self, ActivityError, RunStatus};
use urbaniac::application::economy::{self, SaleOutcome, SellError, SellQty};
use urbaniac::application::{inventory, player};
use urbaniac::domain::activities::ActivityKind;
use urbaniac::domain::resources::find_item;
use urbaniac::infrastructure::repositories::inventory_repo;
use urbaniac::workers::process_due_batch;

/// Memajukan waktu: membuat semua run dan job berjalan jatuh tempo.
async fn make_due(pool: &PgPool) {
    sqlx::query("UPDATE activity_runs SET ends_at = now() - interval '1 second' WHERE status = 'running'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE scheduled_jobs SET run_at = now() - interval '1 second' WHERE status = 'pending'")
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn slots_limit_concurrent_activities(pool: PgPool) {
    player::register(&pool, 10, "a", None).await.unwrap();
    activities::start(&pool, 10, ActivityKind::Farming, 100).await.unwrap();
    activities::start(&pool, 10, ActivityKind::Gathering, 100).await.unwrap();
    let third = activities::start(&pool, 10, ActivityKind::Farming, 100).await;
    assert!(matches!(third, Err(ActivityError::NoFreeSlot(2))));
}

#[sqlx::test(migrations = "./migrations")]
async fn unregistered_player_cannot_start(pool: PgPool) {
    let r = activities::start(&pool, 999, ActivityKind::Farming, 100).await;
    assert!(matches!(r, Err(ActivityError::NotRegistered)));
}

#[sqlx::test(migrations = "./migrations")]
async fn full_activity_flow_survives_restart(pool: PgPool) {
    player::register(&pool, 11, "b", None).await.unwrap();
    activities::start(&pool, 11, ActivityKind::Farming, 100).await.unwrap();

    // Belum jatuh tempo: tidak ada yang diproses.
    assert_eq!(process_due_batch(&pool).await.unwrap(), 0);
    let ov = activities::overview(&pool, 11).await.unwrap();
    assert_eq!(ov.runs[0].status, RunStatus::Running);

    // "Restart bot": job hanya ada di database; batch baru tetap memprosesnya.
    make_due(&pool).await;
    assert_eq!(process_due_batch(&pool).await.unwrap(), 1);
    // Idempoten: tidak diproses dua kali.
    assert_eq!(process_due_batch(&pool).await.unwrap(), 0);

    let ov = activities::overview(&pool, 11).await.unwrap();
    assert_eq!(ov.runs[0].status, RunStatus::Ready);

    let sums = activities::collect_ready(&pool, 11).await.unwrap();
    assert_eq!(sums.len(), 1);
    assert_eq!(sums[0].xp_gain, 10);
    assert!(!sums[0].items.is_empty());

    let inv = inventory::view(&pool, 11).await.unwrap();
    assert!(!inv.lines.is_empty());

    let ov = activities::overview(&pool, 11).await.unwrap();
    assert!(ov.runs.is_empty(), "slot harus bebas setelah reward diambil");
    assert_eq!(ov.progress[0].state.xp, 10);
    assert_eq!(player::get_player(&pool, 11).await.unwrap().unwrap().xp, 10);

    // Klaim kedua tidak memberi apa pun.
    assert!(activities::collect_ready(&pool, 11).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn sell_to_npc_updates_inventory_and_wallet_idempotently(pool: PgPool) {
    player::register(&pool, 12, "c", None).await.unwrap();
    let wheat = find_item("wheat").unwrap(); // 3.00 Bac
    let mut tx = pool.begin().await.unwrap();
    inventory_repo::add_item(&mut *tx, 12, wheat, 10, inventory::slot_capacity(12)).await.unwrap();
    tx.commit().await.unwrap();

    let out = economy::sell_to_npc(&pool, 12, "wheat", SellQty::Exact(4), "sale-1").await.unwrap();
    assert!(matches!(out, SaleOutcome::Sold { qty: 4, .. }));
    assert_eq!(economy::balance(&pool, 12).await.unwrap(), Decimal::new(11_200, 2)); // 100 + 12

    // Key sama: tidak ada perubahan.
    let dup = economy::sell_to_npc(&pool, 12, "wheat", SellQty::Exact(4), "sale-1").await.unwrap();
    assert!(matches!(dup, SaleOutcome::Duplicate));
    assert_eq!(economy::balance(&pool, 12).await.unwrap(), Decimal::new(11_200, 2));
    assert_eq!(inventory::view(&pool, 12).await.unwrap().lines[0].quantity, 6);

    // Terlalu banyak.
    let too_many = economy::sell_to_npc(&pool, 12, "wheat", SellQty::Exact(99), "sale-2").await;
    assert!(matches!(too_many, Err(SellError::NotEnough { have: 6 })));

    // Jual semua menghapus slot.
    economy::sell_to_npc(&pool, 12, "wheat", SellQty::All, "sale-3").await.unwrap();
    assert!(inventory::view(&pool, 12).await.unwrap().lines.is_empty());
    assert_eq!(economy::balance(&pool, 12).await.unwrap(), Decimal::new(13_000, 2)); // 100 + 30
}
