use crate::application::{economy, inventory};
use crate::domain::economy::starter_bac;
use crate::domain::i18n::Lang;
use crate::domain::player::Player;
use crate::infrastructure::repositories::{city_repo, player_repo, wallet_repo};
use anyhow::anyhow;
use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::domain::city::CityStage;

pub struct ProfileView {
    pub player: Player,
    pub balance: Decimal,
    pub city_stage: CityStage,
    pub inventory: inventory::InventoryView,
}

/// create_player: registrasi otomatis lewat Discord User ID. Idempoten.
/// Mengembalikan (player, baru_dibuat). Pemain lama hanya memperbarui username/avatar.
pub async fn register(
    pool: &PgPool,
    id: i64,
    username: &str,
    avatar: Option<&str>,
) -> anyhow::Result<(Player, bool)> {
    let mut tx = pool.begin().await?;
    let created = player_repo::insert_if_absent(&mut *tx, id, username, avatar).await?;
    if created {
        wallet_repo::create(&mut *tx, id).await?;
        wallet_repo::apply(&mut *tx, id, starter_bac(), "starter_grant", &format!("starter:{id}"))
            .await?;
    } else {
        player_repo::update_profile(&mut *tx, id, username, avatar).await?;
    }
    city_repo::create(&mut *tx, id).await?;
    let player = player_repo::get(&mut *tx, id)
        .await?
        .ok_or_else(|| anyhow!("pemain tidak ditemukan setelah registrasi"))?;
    tx.commit().await?;
    Ok((player, created))
}

pub async fn get_player(pool: &PgPool, id: i64) -> anyhow::Result<Option<Player>> {
    let mut conn = pool.acquire().await?;
    Ok(player_repo::get(&mut *conn, id).await?)
}

pub async fn update_player_profile(
    pool: &PgPool,
    id: i64,
    username: &str,
    avatar: Option<&str>,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;
    player_repo::update_profile(&mut *conn, id, username, avatar).await?;
    Ok(())
}

pub async fn get_player_profile(pool: &PgPool, id: i64) -> anyhow::Result<Option<ProfileView>> {
    let Some(player) = get_player(pool, id).await? else { return Ok(None) };
    let balance = economy::balance(pool, id).await?;
    let city_stage = crate::application::city::stage_for(pool, id).await?;
    let inventory = inventory::view(pool, id).await?;
    Ok(Some(ProfileView { player, balance, city_stage, inventory }))
}

pub async fn get_thread_id(pool: &PgPool, id: i64) -> anyhow::Result<Option<u64>> {
    let mut conn = pool.acquire().await?;
    Ok(player_repo::get_thread_id(&mut *conn, id).await?.map(|t| t as u64))
}

pub async fn set_thread_id(pool: &PgPool, id: i64, thread_id: u64) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;
    player_repo::set_thread_id(&mut *conn, id, thread_id as i64).await?;
    Ok(())
}

pub async fn get_lang(pool: &PgPool, id: i64) -> anyhow::Result<Lang> {
    let mut conn = pool.acquire().await?;
    Ok(player_repo::get_lang(&mut *conn, id).await?)
}

pub async fn set_lang(pool: &PgPool, id: i64, lang: Lang) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;
    player_repo::set_lang(&mut *conn, id, lang).await?;
    Ok(())
}

pub async fn toggle_lang(pool: &PgPool, id: i64) -> anyhow::Result<Lang> {
    let mut tx = pool.begin().await?;
    let next = player_repo::get_lang(&mut *tx, id).await?.toggle();
    player_repo::set_lang(&mut *tx, id, next).await?;
    tx.commit().await?;
    Ok(next)
}
