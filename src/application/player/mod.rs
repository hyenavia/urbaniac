use crate::application::{economy, inventory};
use crate::domain::economy::starter_bac;
use crate::domain::player::Player;
use crate::infrastructure::repositories::{player_repo, wallet_repo};
use anyhow::anyhow;
use rust_decimal::Decimal;
use sqlx::PgPool;

pub struct ProfileView {
    pub player: Player,
    pub balance: Decimal,
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
    let inventory = inventory::view(pool, id).await?;
    Ok(Some(ProfileView { player, balance, inventory }))
}
