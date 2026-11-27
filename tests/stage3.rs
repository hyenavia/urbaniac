//! Integration test tahap 3 (Kota, Treasury, Proyek, Lumbung, Stage Cap).
//! Butuh PostgreSQL (DATABASE_URL, user boleh CREATE DATABASE).
use rust_decimal::Decimal;
use sqlx::PgPool;
use urbaniac::application::city::{self, CityError};
use urbaniac::application::{economy, inventory, player};
use urbaniac::domain::city::{CityStage, ProjectId};
use urbaniac::domain::progression::activity_level_cap;
use urbaniac::domain::resources::find_item;
use urbaniac::infrastructure::repositories::inventory_repo;
use urbaniac::workers::process_due_batch;

async fn make_projects_due(pool: &PgPool) {
    sqlx::query("UPDATE city_projects SET ends_at = now() - interval '1 second' WHERE status = 'running'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE scheduled_jobs SET run_at = now() - interval '1 second' WHERE status = 'pending'")
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn register_creates_kampung_city_and_treasury_is_separate(pool: PgPool) {
    player::register(&pool, 100, "mayor", None).await.unwrap();
    let ov = city::overview(&pool, 100).await.unwrap();
    assert_eq!(ov.stage, CityStage::Kampung);
    assert_eq!(ov.treasury, Decimal::ZERO);
    assert_eq!(ov.wallet_balance, Decimal::new(10_000, 2));

    let (w, t) = city::deposit_to_treasury(&pool, 100, Decimal::new(2_500, 2), "dep1")
        .await
        .unwrap();
    assert_eq!(w, Decimal::new(7_500, 2));
    assert_eq!(t, Decimal::new(2_500, 2));

    let too_much = city::deposit_to_treasury(&pool, 100, Decimal::new(99_999, 2), "dep2").await;
    assert!(matches!(too_much, Err(CityError::InsufficientBac { .. })));
    assert_eq!(economy::balance(&pool, 100).await.unwrap(), Decimal::new(7_500, 2));
    assert_eq!(city::overview(&pool, 100).await.unwrap().treasury, Decimal::new(2_500, 2));
}

#[sqlx::test(migrations = "./migrations")]
async fn project_rejects_without_mutating_wallet_or_inventory(pool: PgPool) {
    player::register(&pool, 101, "p1", None).await.unwrap();
    // Wallet punya 100 Bac (cukup untuk Sumur 50 Bac), tapi Kayu masih 0.
    let err = city::start_project(&pool, 101, ProjectId::Well, 100, "proj1").await;
    assert!(matches!(err, Err(CityError::InsufficientItem { .. })));
    assert_eq!(economy::balance(&pool, 101).await.unwrap(), Decimal::new(10_000, 2));

    // Beri Kayu cukup, tapi habiskan Wallet dulu.
    let wood = find_item("wood").unwrap();
    let mut tx = pool.begin().await.unwrap();
    inventory_repo::add_item(&mut *tx, 101, wood, 20, 12).await.unwrap();
    tx.commit().await.unwrap();
    economy::apply(&pool, 101, Decimal::new(-9_000, 2), "spend", "s1")
        .await
        .unwrap();

    let err2 = city::start_project(&pool, 101, ProjectId::Well, 100, "proj2").await;
    assert!(matches!(err2, Err(CityError::InsufficientBac { .. })));
    assert_eq!(economy::balance(&pool, 101).await.unwrap(), Decimal::new(1_000, 2));
    assert_eq!(inventory::view(&pool, 101).await.unwrap().lines[0].quantity, 20);
}

#[sqlx::test(migrations = "./migrations")]
async fn projects_complete_via_scheduler_and_advance_stage(pool: PgPool) {
    player::register(&pool, 102, "builder", None).await.unwrap();
    // Tambah Bac & resource cukup untuk Sumur, Lumbung, dan Balai Kampung.
    economy::apply(&pool, 102, Decimal::new(50_000, 2), "grant", "g1")
        .await
        .unwrap();
    let wood = find_item("wood").unwrap();
    let wheat = find_item("wheat").unwrap();
    let potato = find_item("potato").unwrap();
    let mut tx = pool.begin().await.unwrap();
    inventory_repo::add_item(&mut *tx, 102, wood, 60, 12).await.unwrap();
    inventory_repo::add_item(&mut *tx, 102, wheat, 20, 12).await.unwrap();
    inventory_repo::add_item(&mut *tx, 102, potato, 20, 12).await.unwrap();
    tx.commit().await.unwrap();

    // Balai Kampung ditolak sebelum Sumur & Lumbung selesai.
    let early = city::start_project(&pool, 102, ProjectId::TownHall, 100, "th0").await;
    assert!(matches!(early, Err(CityError::MissingPrerequisites)));

    // Bangun Sumur -> selesaikan lewat scheduler.
    city::start_project(&pool, 102, ProjectId::Well, 100, "w1").await.unwrap();
    make_projects_due(&pool).await;
    assert_eq!(process_due_batch(&pool).await.unwrap(), 1);

    // Bangun Lumbung -> kapasitas inventory naik dari 12 menjadi 16.
    assert_eq!(inventory::slot_capacity(&pool, 102).await.unwrap(), 12);
    city::start_project(&pool, 102, ProjectId::Granary, 100, "gr1").await.unwrap();
    make_projects_due(&pool).await;
    assert_eq!(process_due_batch(&pool).await.unwrap(), 1);
    assert_eq!(inventory::slot_capacity(&pool, 102).await.unwrap(), 16);

    // Bangun Balai Kampung -> stage naik dari Kampung ke Desa, level cap naik 10 -> 20.
    assert_eq!(activity_level_cap(city::stage_for(&pool, 102).await.unwrap()), 10);
    city::start_project(&pool, 102, ProjectId::TownHall, 100, "th1").await.unwrap();
    make_projects_due(&pool).await;
    assert_eq!(process_due_batch(&pool).await.unwrap(), 1);

    let stage = city::stage_for(&pool, 102).await.unwrap();
    assert_eq!(stage, CityStage::Desa);
    assert_eq!(activity_level_cap(stage), 20);
}
