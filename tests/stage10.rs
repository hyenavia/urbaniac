//! Integration test Fase 8 (Milestone 10: Global Calendar, Musim, Event, Tren).
//! Butuh PostgreSQL (DATABASE_URL, user boleh CREATE DATABASE).
use rust_decimal::Decimal;
use sqlx::PgPool;
use urbaniac::application::economy::{self, SaleOutcome, SellQty};
use urbaniac::application::{market, player};
use urbaniac::domain::resources::find_item;
use urbaniac::domain::world::{Season, WorldEvent};
use urbaniac::infrastructure::repositories::inventory_repo;

#[sqlx::test(migrations = "./migrations")]
async fn global_calendar_season_event_and_trend_affect_market_prices(pool: PgPool) {
    player::register(&pool, 800, "merchant", None).await.unwrap();
    let wheat = find_item("wheat").unwrap(); // base 3.00 Bac
    assert_eq!(market::current_npc_price(&pool, wheat).await.unwrap(), Decimal::new(300, 2));

    // Musim Gugur (1.15x) + Festival Panen (1.20x) + Tren Gandum (1.15x) => 1.5870x => 4.76 Bac
    let cal = market::advance_world_day(
        &pool,
        Some(Season::Autumn),
        Some(WorldEvent::HarvestFestival),
        Some("wheat"),
    )
    .await
    .unwrap();
    assert_eq!(cal.day, 2);
    assert_eq!(cal.season, Season::Autumn);
    assert_eq!(market::current_npc_price(&pool, wheat).await.unwrap(), Decimal::new(476, 2));

    // Jual 10 wheat ke NPC memakai harga kalender baru
    let mut tx = pool.begin().await.unwrap();
    inventory_repo::add_item(&mut *tx, 800, wheat, 10, 12).await.unwrap();
    tx.commit().await.unwrap();
    let sale = economy::sell_to_npc(&pool, 800, "wheat", SellQty::Exact(10), "cal_sale")
        .await
        .unwrap();
    assert!(matches!(
        sale,
        SaleOutcome::Sold { total, .. } if total == Decimal::new(4_760, 2)
    ));
}
