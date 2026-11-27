//! Integration test. Butuh PostgreSQL: set DATABASE_URL (user harus boleh CREATE DATABASE).
use rust_decimal::Decimal;
use sqlx::PgPool;
use urbaniac::application::{economy, inventory, player};
use urbaniac::domain::resources::find_item;
use urbaniac::infrastructure::repositories::inventory_repo;
use urbaniac::infrastructure::repositories::wallet_repo::{ApplyOutcome, WalletError};

#[sqlx::test(migrations = "./migrations")]
async fn register_creates_profile_and_starter_wallet_once(pool: PgPool) {
    let (p, created) = player::register(&pool, 1, "ani", None).await.unwrap();
    assert!(created);
    assert_eq!(p.level, 1);
    assert_eq!(economy::balance(&pool, 1).await.unwrap(), Decimal::new(10_000, 2));

    let (_, created2) = player::register(&pool, 1, "ani2", None).await.unwrap();
    assert!(!created2);
    assert_eq!(economy::balance(&pool, 1).await.unwrap(), Decimal::new(10_000, 2));
    assert_eq!(player::get_player(&pool, 1).await.unwrap().unwrap().username, "ani2");
}

#[sqlx::test(migrations = "./migrations")]
async fn wallet_is_idempotent_and_never_negative(pool: PgPool) {
    player::register(&pool, 2, "b", None).await.unwrap();
    let plus = Decimal::new(500, 2);

    let first = economy::apply(&pool, 2, plus, "test", "k1").await.unwrap();
    assert!(matches!(first, ApplyOutcome::Applied { .. }));
    let dup = economy::apply(&pool, 2, plus, "test", "k1").await.unwrap();
    assert_eq!(dup, ApplyOutcome::Duplicate);
    assert_eq!(economy::balance(&pool, 2).await.unwrap(), Decimal::new(10_500, 2));

    let too_much = economy::apply(&pool, 2, Decimal::new(-99_999, 2), "test", "k2").await;
    assert!(matches!(too_much, Err(WalletError::InsufficientFunds)));
    assert_eq!(economy::balance(&pool, 2).await.unwrap(), Decimal::new(10_500, 2));
}

#[sqlx::test(migrations = "./migrations")]
async fn overflow_goes_to_pending_and_can_be_claimed(pool: PgPool) {
    player::register(&pool, 3, "c", None).await.unwrap();
    let wheat = find_item("wheat").unwrap(); // stack 100
    let cap = inventory::slot_capacity(&pool, 3).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let p = inventory_repo::add_item(&mut *tx, 3, wheat, 130, cap).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!((p.stored, p.pending), (100, 30));

    let v = inventory::view(&pool, 3).await.unwrap();
    assert_eq!(v.lines[0].quantity, 100);
    assert_eq!(v.pending[0].quantity, 30);

    // Belum ada ruang: klaim tidak memindahkan apa pun.
    let r = inventory::claim_pending(&pool, 3).await.unwrap();
    assert_eq!((r[0].claimed, r[0].remaining), (0, 30));

    // Kosongkan 50, lalu klaim.
    let mut tx = pool.begin().await.unwrap();
    assert!(inventory_repo::remove_item(&mut *tx, 3, "wheat", 50).await.unwrap());
    tx.commit().await.unwrap();
    let r = inventory::claim_pending(&pool, 3).await.unwrap();
    assert_eq!((r[0].claimed, r[0].remaining), (30, 0));

    let v = inventory::view(&pool, 3).await.unwrap();
    assert_eq!(v.lines[0].quantity, 80);
    assert!(v.pending.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn remove_item_fails_when_insufficient(pool: PgPool) {
    player::register(&pool, 4, "d", None).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    assert!(!inventory_repo::remove_item(&mut *tx, 4, "wheat", 1).await.unwrap());
}
