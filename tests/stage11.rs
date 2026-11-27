//! Integration test Fase 9 (Milestone 11: Progression Rewards & Cross-Stage Overflow Flush).
//! Butuh PostgreSQL (DATABASE_URL, user boleh CREATE DATABASE).
use rust_decimal::Decimal;
use sqlx::PgPool;
use urbaniac::application::{activities, city, economy, player};
use urbaniac::domain::activities::ActivityKind;
use urbaniac::domain::city::{CityStage, ProjectId};
use urbaniac::domain::progression::XpState;
use urbaniac::domain::resources::find_item;
use urbaniac::infrastructure::repositories::{activity_repo, inventory_repo, player_repo};
use urbaniac::workers::process_due_batch;

#[sqlx::test(migrations = "./migrations")]
async fn cross_stage_overflow_flushes_on_upgrade_and_player_level_grants_reward_once(pool: PgPool) {
    player::register(&pool, 900, "veteran", None).await.unwrap();
    economy::apply(&pool, 900, Decimal::new(100_000, 2), "grant", "g1")
        .await
        .unwrap();

    // Siapkan Sumur + Lumbung + bahan Balai Kampung + overflow XP di Farming (cap Kampung = 10)
    let mut tx = pool.begin().await.unwrap();
    activity_repo::lock_progress(&mut *tx, 900, "farming").await.unwrap();
    activity_repo::save_progress(
        &mut *tx,
        900,
        "farming",
        XpState { level: 10, xp: 0, overflow: 2_500 },
    )
    .await
    .unwrap();
    // Set player hampir Lv 5 (Lv 4, xp 395 / 400)
    player_repo::set_progress(&mut *tx, 900, 4, 395).await.unwrap();
    let wood = find_item("wood").unwrap();
    let potato = find_item("potato").unwrap();
    inventory_repo::add_item(&mut *tx, 900, wood, 30, 12).await.unwrap();
    inventory_repo::add_item(&mut *tx, 900, potato, 15, 12).await.unwrap();
    sqlx::query("INSERT INTO city_facilities (player_id, facility_id, level) VALUES (900, 'well', 1), (900, 'granary', 1)")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // Mulai Balai Kampung -> selesaikan lewat scheduler -> naik ke Desa & overflow Farming (2500 XP) langsung naikkan level > 10!
    city::start_project(&pool, 900, ProjectId::TownHall, 100, "th_ov")
        .await
        .unwrap();
    sqlx::query("UPDATE city_projects SET ends_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE scheduled_jobs SET run_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    process_due_batch(&pool).await.unwrap();

    assert_eq!(city::stage_for(&pool, 900).await.unwrap(), CityStage::Desa);
    let ov = activities::overview(&pool, 900).await.unwrap();
    let farming = ov.progress.iter().find(|p| p.kind == ActivityKind::Farming).unwrap();
    assert!(farming.state.level > 10);
    assert_eq!(farming.state.overflow, 0);

    // Selesaikan 1 aktivitas -> pemain naik ke Lv 5 -> dapat reward 100.00 Bac sekali saja
    let bal_before = economy::balance(&pool, 900).await.unwrap();
    activities::start(&pool, 900, ActivityKind::Farming, 100).await.unwrap();
    sqlx::query("UPDATE activity_runs SET ends_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE scheduled_jobs SET run_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    process_due_batch(&pool).await.unwrap();
    activities::collect_ready(&pool, 900).await.unwrap();

    let p = player::get_player(&pool, 900).await.unwrap().unwrap();
    assert_eq!(p.level, 5);
    assert_eq!(economy::balance(&pool, 900).await.unwrap(), bal_before + Decimal::new(10_000, 2));
}
