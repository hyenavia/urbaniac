//! Integration test Fase 3 (Milestone 5: Crafting & Pesanan NPC).
//! Butuh PostgreSQL (DATABASE_URL, user boleh CREATE DATABASE).
use rust_decimal::Decimal;
use sqlx::PgPool;
use urbaniac::application::crafting::{self, CraftError};
use urbaniac::application::{economy, inventory, player};
use urbaniac::domain::resources::find_item;
use urbaniac::infrastructure::repositories::{crafting_repo, inventory_repo};
use urbaniac::workers::process_due_batch;

#[sqlx::test(migrations = "./migrations")]
async fn craft_consumes_inputs_and_produces_output_via_scheduler(pool: PgPool) {
    player::register(&pool, 300, "crafter", None).await.unwrap();
    let wheat = find_item("wheat").unwrap();
    let mut tx = pool.begin().await.unwrap();
    inventory_repo::add_item(&mut *tx, 300, wheat, 5, 12).await.unwrap();
    tx.commit().await.unwrap();

    // Kurang bahan ditolak tanpa mutasi
    let err = crafting::start_craft(&pool, 300, "bread", 100).await;
    assert!(matches!(err, Err(CraftError::MissingIngredient { .. })));

    // Craft flour (butuh wheat x2)
    crafting::start_craft(&pool, 300, "flour", 100).await.unwrap();
    assert_eq!(inventory::view(&pool, 300).await.unwrap().lines[0].quantity, 3);

    sqlx::query("UPDATE crafting_jobs SET ends_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE scheduled_jobs SET run_at = now() - interval '1 second' WHERE kind = 'craft_complete'")
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(process_due_batch(&pool).await.unwrap(), 1);
    let inv = inventory::view(&pool, 300).await.unwrap();
    assert!(inv.lines.iter().any(|l| l.item.id == "flour" && l.quantity == 1));
}

#[sqlx::test(migrations = "./migrations")]
async fn fulfill_npc_order_grants_bac_and_reputation(pool: PgPool) {
    player::register(&pool, 301, "baker", None).await.unwrap();
    let bread = find_item("bread").unwrap();
    let mut tx = pool.begin().await.unwrap();
    inventory_repo::add_item(&mut *tx, 301, bread, 2, 12).await.unwrap();
    let order = crafting_repo::insert_order(
        &mut *tx,
        301,
        "bread",
        2,
        Decimal::new(5000, 2), // 50.00 Bac
        50,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let sum = crafting::fulfill_order(&pool, 301, order.id).await.unwrap();
    assert_eq!(sum.reward_bac, Decimal::new(5000, 2));
    assert_eq!(sum.new_rep_level, 2);
    assert_eq!(economy::balance(&pool, 301).await.unwrap(), Decimal::new(15_000, 2));
    assert!(inventory::view(&pool, 301).await.unwrap().lines.is_empty());
}
