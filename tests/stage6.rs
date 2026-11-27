//! Integration test Fase 4 (Milestone 6: Kru & Automasi).
//! Butuh PostgreSQL (DATABASE_URL, user boleh CREATE DATABASE).
use rust_decimal::Decimal;
use sqlx::PgPool;
use urbaniac::application::{activities, crew, economy, inventory, player};
use urbaniac::domain::activities::{ActivityKind, BASE_ACTIVITY_SLOTS};
use urbaniac::workers::process_due_batch;

#[sqlx::test(migrations = "./migrations")]
async fn crew_recruit_upgrade_and_automate_without_adding_slots(pool: PgPool) {
    player::register(&pool, 400, "boss", None).await.unwrap();
    economy::apply(&pool, 400, Decimal::new(50_000, 2), "grant", "g1")
        .await
        .unwrap();

    let cw = crew::recruit(&pool, 400, "hire1").await.unwrap();
    let upg = crew::upgrade(&pool, 400, cw.id, "upg1").await.unwrap();
    assert_eq!(upg.level, 2);

    // Tugaskan ke Farming dengan AUTO = true
    crew::assign(&pool, 400, cw.id, Some(ActivityKind::Farming), true)
        .await
        .unwrap();

    // Mulai 1 Farming, lalu buat jatuh tempo
    activities::start(&pool, 400, ActivityKind::Farming, 100)
        .await
        .unwrap();
    sqlx::query("UPDATE activity_runs SET ends_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE scheduled_jobs SET run_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(process_due_batch(&pool).await.unwrap(), 1);

    // Reward otomatis masuk inventory & 1 run baru Farming otomatis berjalan (tetap 1 slot terpakai, <= BASE_ACTIVITY_SLOTS)
    let inv = inventory::view(&pool, 400).await.unwrap();
    assert!(!inv.lines.is_empty());
    let ov = activities::overview(&pool, 400).await.unwrap();
    assert_eq!(ov.slots_total, BASE_ACTIVITY_SLOTS);
    assert_eq!(ov.runs.len(), 1);
    assert_eq!(ov.runs[0].status, activities::RunStatus::Running);
}
