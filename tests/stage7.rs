//! Integration test Fase 5 (Milestone 7: Activity & Resource Expansion).
//! Butuh PostgreSQL (DATABASE_URL, user boleh CREATE DATABASE).
use sqlx::PgPool;
use urbaniac::application::activities::{self, ActivityError};
use urbaniac::application::player;
use urbaniac::domain::activities::ActivityKind;
use urbaniac::domain::city::CityStage;
use urbaniac::infrastructure::repositories::city_repo;

#[sqlx::test(migrations = "./migrations")]
async fn fishing_and_mining_unlock_by_city_stage(pool: PgPool) {
    player::register(&pool, 500, "explorer", None).await.unwrap();

    // Kampung: Fishing & Mining terkunci
    let f_locked = activities::start(&pool, 500, ActivityKind::Fishing, 100).await;
    assert!(matches!(f_locked, Err(ActivityError::Locked(CityStage::Desa))));
    let m_locked = activities::start(&pool, 500, ActivityKind::Mining, 100).await;
    assert!(matches!(m_locked, Err(ActivityError::Locked(CityStage::KotaKecil))));

    // Naik ke Desa: Fishing terbuka, Mining masih terkunci
    let mut conn = pool.acquire().await.unwrap();
    city_repo::set_stage(&mut *conn, 500, CityStage::Desa.as_str()).await.unwrap();
    drop(conn);

    activities::start(&pool, 500, ActivityKind::Fishing, 100).await.unwrap();
    let m_still_locked = activities::start(&pool, 500, ActivityKind::Mining, 100).await;
    assert!(matches!(m_still_locked, Err(ActivityError::Locked(CityStage::KotaKecil))));

    // Naik ke Kota Kecil: Mining terbuka
    let mut conn = pool.acquire().await.unwrap();
    city_repo::set_stage(&mut *conn, 500, CityStage::KotaKecil.as_str()).await.unwrap();
    drop(conn);

    activities::start(&pool, 500, ActivityKind::Mining, 100).await.unwrap();
}
