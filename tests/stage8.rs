//! Integration test Fase 6 (Milestone 8: City Development Desa -> Metropolis).
//! Butuh PostgreSQL (DATABASE_URL, user boleh CREATE DATABASE).
use rust_decimal::Decimal;
use sqlx::PgPool;
use urbaniac::application::{city, economy, inventory, player};
use urbaniac::domain::city::{CityStage, ProjectId};
use urbaniac::domain::progression::activity_level_cap;
use urbaniac::domain::resources::find_item;
use urbaniac::infrastructure::repositories::{city_repo, inventory_repo};
use urbaniac::workers::process_due_batch;

async fn finish_city_jobs(pool: &PgPool) {
    sqlx::query("UPDATE city_projects SET ends_at = now() - interval '1 second' WHERE status = 'running'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE scheduled_jobs SET run_at = now() - interval '1 second' WHERE status = 'pending'")
        .execute(pool)
        .await
        .unwrap();
    process_due_batch(pool).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn city_upgrades_from_desa_to_metropolis_and_granary_scales(pool: PgPool) {
    player::register(&pool, 600, "architect", None).await.unwrap();
    economy::apply(&pool, 600, Decimal::new(500_000, 2), "grant", "g1")
        .await
        .unwrap();

    // Beri banyak resource & mulai dari Desa
    let mut conn = pool.acquire().await.unwrap();
    city_repo::set_stage(&mut *conn, 600, CityStage::Desa.as_str()).await.unwrap();
    drop(conn);

    let mut tx = pool.begin().await.unwrap();
    for (id, qty) in [
        ("wood", 80),
        ("wheat", 60),
        ("plank", 40),
        ("bread", 10),
        ("wooden_crate", 10),
        ("river_fish", 20),
        ("iron_ore", 20),
        ("amethyst", 10),
        ("healing_tonic", 10),
    ] {
        inventory_repo::add_item(&mut *tx, 600, find_item(id).unwrap(), qty, 20)
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();

    // Upgrade Lumbung Lv 1 -> Lv 2 (kapasitas 12 -> 16 -> 20)
    city::start_project(&pool, 600, ProjectId::Granary, 100, "gr1").await.unwrap();
    finish_city_jobs(&pool).await;
    assert_eq!(inventory::slot_capacity(&pool, 600).await.unwrap(), 16);

    city::start_project(&pool, 600, ProjectId::Granary, 100, "gr2").await.unwrap();
    finish_city_jobs(&pool).await;
    assert_eq!(inventory::slot_capacity(&pool, 600).await.unwrap(), 20);

    // Desa -> Kota Kecil -> Kota Dagang -> Metropolis
    city::start_project(&pool, 600, ProjectId::MarketHall, 100, "mh").await.unwrap();
    finish_city_jobs(&pool).await;
    city::start_project(&pool, 600, ProjectId::CityHall, 100, "ch").await.unwrap();
    finish_city_jobs(&pool).await;
    assert_eq!(city::stage_for(&pool, 600).await.unwrap(), CityStage::KotaKecil);
    assert_eq!(activity_level_cap(CityStage::KotaKecil), 30);

    city::start_project(&pool, 600, ProjectId::TradePort, 100, "tp").await.unwrap();
    finish_city_jobs(&pool).await;
    assert_eq!(city::stage_for(&pool, 600).await.unwrap(), CityStage::KotaDagang);
    assert_eq!(activity_level_cap(CityStage::KotaDagang), 40);

    city::start_project(&pool, 600, ProjectId::MetropolisTower, 100, "mt").await.unwrap();
    finish_city_jobs(&pool).await;
    assert_eq!(city::stage_for(&pool, 600).await.unwrap(), CityStage::Metropolis);
    assert_eq!(activity_level_cap(CityStage::Metropolis), 50);
}
