//! Integration test Fase 2 (Milestone 4: Pasar & Makelar).
//! Butuh PostgreSQL (DATABASE_URL, user boleh CREATE DATABASE).
use rust_decimal::Decimal;
use sqlx::PgPool;
use urbaniac::application::market::{self, MarketError};
use urbaniac::application::{economy, inventory, player};
use urbaniac::domain::market::ListingDuration;
use urbaniac::domain::resources::find_item;
use urbaniac::infrastructure::repositories::inventory_repo;
use urbaniac::workers::process_due_batch;

#[sqlx::test(migrations = "./migrations")]
async fn broker_rejects_without_mutating_wallet_or_inventory(pool: PgPool) {
    player::register(&pool, 200, "m1", None).await.unwrap();
    let wheat = find_item("wheat").unwrap();
    let mut tx = pool.begin().await.unwrap();
    inventory_repo::add_item(&mut *tx, 200, wheat, 5, 12).await.unwrap();
    tx.commit().await.unwrap();

    // Kurang item
    let err1 = market::create_listing(
        &pool,
        200,
        "wheat",
        10,
        Decimal::new(350, 2),
        ListingDuration::Short,
        100,
        "b1",
    )
    .await;
    assert!(matches!(err1, Err(MarketError::InsufficientItem { have: 5 })));
    assert_eq!(economy::balance(&pool, 200).await.unwrap(), Decimal::new(10_000, 2));

    // Habiskan wallet sehingga tidak cukup untuk fee min 0.50 Bac
    economy::apply(&pool, 200, Decimal::new(-9_980, 2), "spend", "s1")
        .await
        .unwrap();
    let err2 = market::create_listing(
        &pool,
        200,
        "wheat",
        5,
        Decimal::new(350, 2),
        ListingDuration::Short,
        100,
        "b2",
    )
    .await;
    assert!(matches!(err2, Err(MarketError::InsufficientBacForFee { .. })));
    assert_eq!(inventory::view(&pool, 200).await.unwrap().lines[0].quantity, 5);
}

#[sqlx::test(migrations = "./migrations")]
async fn broker_listing_sells_on_market_tick_with_fee_and_commission(pool: PgPool) {
    player::register(&pool, 201, "seller", None).await.unwrap();
    let wheat = find_item("wheat").unwrap(); // base 3.00 Bac
    let mut tx = pool.begin().await.unwrap();
    inventory_repo::add_item(&mut *tx, 201, wheat, 10, 12).await.unwrap();
    tx.commit().await.unwrap();

    // Target 3.20 Bac/unit, total 32.00 -> fee min 0.50 Bac. Wallet 100 -> 99.50 Bac.
    market::create_listing(
        &pool,
        201,
        "wheat",
        10,
        Decimal::new(320, 2),
        ListingDuration::Medium,
        100,
        "b_sell",
    )
    .await
    .unwrap();
    assert_eq!(economy::balance(&pool, 201).await.unwrap(), Decimal::new(9_950, 2));
    assert!(inventory::view(&pool, 201).await.unwrap().lines.is_empty());

    // Naikkan harga pasar (+800 bps => 1.0800x => 3.24 Bac >= target 3.20 Bac)
    let mut tx = pool.begin().await.unwrap();
    let sold = market::run_market_tick_conn(&mut *tx, Some(800)).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(sold, 1);

    // Gross 32.40, komisi 3% = 0.97, net = 31.43 -> Wallet 99.50 + 31.43 = 130.93 Bac
    assert_eq!(economy::balance(&pool, 201).await.unwrap(), Decimal::new(13_093, 2));
    assert!(market::overview(&pool, 201).await.unwrap().listings.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn expired_listing_returns_items_via_scheduler(pool: PgPool) {
    player::register(&pool, 202, "exp", None).await.unwrap();
    let wood = find_item("wood").unwrap();
    let mut tx = pool.begin().await.unwrap();
    inventory_repo::add_item(&mut *tx, 202, wood, 8, 12).await.unwrap();
    tx.commit().await.unwrap();

    market::create_listing(
        &pool,
        202,
        "wood",
        8,
        Decimal::new(400, 2),
        ListingDuration::Short,
        100,
        "b_exp",
    )
    .await
    .unwrap();
    assert!(inventory::view(&pool, 202).await.unwrap().lines.is_empty());

    sqlx::query("UPDATE broker_listings SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE scheduled_jobs SET run_at = now() - interval '1 second' WHERE kind = 'broker_expire'")
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(process_due_batch(&pool).await.unwrap(), 1);
    let inv = inventory::view(&pool, 202).await.unwrap();
    assert_eq!(inv.lines[0].item.id, "wood");
    assert_eq!(inv.lines[0].quantity, 8);
}
