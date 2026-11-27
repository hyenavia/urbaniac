//! Integration test Fase 7 (Milestone 9: City Economy, Tax, Maintenance, Welfare).
//! Butuh PostgreSQL (DATABASE_URL, user boleh CREATE DATABASE).
use rust_decimal::Decimal;
use sqlx::PgPool;
use urbaniac::application::{city, player};
use urbaniac::infrastructure::repositories::city_repo;

#[sqlx::test(migrations = "./migrations")]
async fn citizen_tax_and_empty_treasury_degrades_welfare_without_negative_balance(pool: PgPool) {
    player::register(&pool, 700, "treasurer", None).await.unwrap();

    // Tambah fasilitas & atur pajak 15%
    let mut conn = pool.acquire().await.unwrap();
    city_repo::upsert_facility(&mut *conn, 700, "well").await.unwrap();
    city_repo::upsert_facility(&mut *conn, 700, "granary").await.unwrap();
    drop(conn);

    city::set_tax_rate(&pool, 700, 15).await.unwrap();
    city::deposit_to_treasury(&pool, 700, Decimal::new(5_000, 2), "d1")
        .await
        .unwrap();

    // Tick 1 dengan Treasury cukup -> welfare tetap terjaga, treasury tidak negatif
    let mut tx = pool.begin().await.unwrap();
    let notes = city::run_city_economy_tick_conn(&mut *tx, 1).await.unwrap();
    tx.commit().await.unwrap();
    assert!(notes.is_empty());

    let ov1 = city::overview(&pool, 700).await.unwrap();
    assert_eq!(ov1.tax_rate_pct, 15);
    assert!(ov1.treasury >= Decimal::ZERO);
    assert!(ov1.effectiveness_pct >= 100);

    // Nolkan pajak & kosongkan treasury -> shortfall menurunkan welfare & effectiveness tanpa saldo negatif
    city::set_tax_rate(&pool, 700, 0).await.unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let cur = city_repo::get(&mut *conn, 700).await.unwrap().unwrap().treasury;
    if cur > Decimal::ZERO {
        city_repo::apply_treasury(&mut *conn, 700, -cur, "drain", "drain1")
            .await
            .unwrap();
    }
    drop(conn);

    let mut tx = pool.begin().await.unwrap();
    let notes2 = city::run_city_economy_tick_conn(&mut *tx, 2).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(notes2.len(), 1);

    let ov2 = city::overview(&pool, 700).await.unwrap();
    assert_eq!(ov2.treasury, Decimal::ZERO);
    assert!(ov2.welfare < 100);
    assert!(ov2.effectiveness_pct < 115);
}
