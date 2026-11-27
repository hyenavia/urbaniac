use crate::application::{inventory, market};
use crate::domain::crafting::{
    apply_rep_bonus, apply_rep_xp, find_recipe, rep_bonus_pct, rep_xp_to_next, RecipeDef,
    MAX_ACTIVE_ORDERS, MAX_CONCURRENT_CRAFTS, RECIPES,
};
use crate::domain::economy::npc_buy_price;
use crate::domain::resources::{find_item, ItemDef};
use crate::infrastructure::repositories::wallet_repo::{self, ApplyOutcome, WalletError};
use crate::infrastructure::repositories::{crafting_repo, inventory_repo, player_repo};
use crate::infrastructure::scheduler;
use chrono::{DateTime, Utc};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rust_decimal::Decimal;
use sqlx::{PgConnection, PgPool};

pub const JOB_CRAFT_COMPLETE: &str = "craft_complete";

#[derive(Debug, thiserror::Error)]
pub enum CraftError {
    #[error("profil belum ada, jalankan /start")]
    NotRegistered,
    #[error("resep tidak dikenal")]
    UnknownRecipe,
    #[error("maksimal {0} crafting berjalan bersamaan")]
    TooManyCrafts(i64),
    #[error("bahan {name} tidak cukup (butuh {need}, punya {have})")]
    MissingIngredient { name: &'static str, need: i64, have: i64 },
    #[error("pesanan tidak ditemukan atau sudah selesai")]
    OrderNotFound,
    #[error("item {name} kurang untuk pesanan (butuh {need}, punya {have})")]
    InsufficientOrderItem { name: &'static str, need: i64, have: i64 },
    #[error(transparent)]
    Wallet(#[from] WalletError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

pub struct CraftRunView {
    pub id: i64,
    pub recipe: &'static RecipeDef,
    pub quantity: i64,
    pub auto_sell: bool,
    pub ends_at: DateTime<Utc>,
}

pub struct OrderView {
    pub id: i64,
    pub item: &'static ItemDef,
    pub quantity: i64,
    pub reward_bac: Decimal,
    pub rep_xp: i64,
}

pub struct CraftOverview {
    pub running: Vec<CraftRunView>,
    pub orders: Vec<OrderView>,
    pub rep_level: i32,
    pub rep_xp: i64,
    pub rep_xp_needed: i64,
    pub rep_bonus_pct: i64,
    pub completed_orders: i64,
    pub auto_sell: bool,
}

pub struct OrderFulfillSummary {
    pub item: &'static ItemDef,
    pub quantity: i64,
    pub reward_bac: Decimal,
    pub rep_xp_gain: i64,
    pub old_rep_level: i32,
    pub new_rep_level: i32,
}

async fn ensure_orders_conn(conn: &mut PgConnection, player_id: i64) -> sqlx::Result<()> {
    let existing = crafting_repo::list_active_orders(conn, player_id).await?;
    let need = (MAX_ACTIVE_ORDERS - existing.len() as i64).max(0);
    if need == 0 {
        return Ok(());
    }
    let mut rng = StdRng::from_entropy();
    for _ in 0..need {
        let idx = rng.gen_range(0..RECIPES.len());
        let recipe = &RECIPES[idx];
        let Some(item) = find_item(recipe.output_item_id) else { continue };
        let qty = rng.gen_range(1..=3);
        let base = npc_buy_price(item) * Decimal::from(qty);
        let reward = (base * Decimal::new(125, 2)).round_dp(2); // +25% dibanding NPC langsung
        let rep_xp = 15 * qty;
        crafting_repo::insert_order(conn, player_id, item.id, qty, reward, rep_xp).await?;
    }
    Ok(())
}

pub async fn overview(pool: &PgPool, player_id: i64) -> anyhow::Result<CraftOverview> {
    let mut tx = pool.begin().await?;
    let rep = crafting_repo::ensure_shop_rep(&mut *tx, player_id).await?;
    ensure_orders_conn(&mut *tx, player_id).await?;
    let running = crafting_repo::list_running_crafts(&mut *tx, player_id)
        .await?
        .into_iter()
        .filter_map(|r| {
            find_recipe(&r.recipe_id).map(|recipe| CraftRunView {
                id: r.id,
                recipe,
                quantity: r.quantity,
                auto_sell: r.auto_sell,
                ends_at: r.ends_at,
            })
        })
        .collect();
    let orders = crafting_repo::list_active_orders(&mut *tx, player_id)
        .await?
        .into_iter()
        .filter_map(|o| {
            find_item(&o.item_id).map(|item| OrderView {
                id: o.id,
                item,
                quantity: o.quantity,
                reward_bac: apply_rep_bonus(o.reward_bac, rep.level),
                rep_xp: o.rep_xp,
            })
        })
        .collect();
    tx.commit().await?;

    Ok(CraftOverview {
        running,
        orders,
        rep_level: rep.level,
        rep_xp: rep.xp,
        rep_xp_needed: rep_xp_to_next(rep.level),
        rep_bonus_pct: rep_bonus_pct(rep.level),
        completed_orders: rep.completed_orders,
        auto_sell: rep.auto_sell,
    })
}

pub async fn toggle_auto_sell(pool: &PgPool, player_id: i64) -> anyhow::Result<bool> {
    let mut tx = pool.begin().await?;
    let _ = crafting_repo::ensure_shop_rep(&mut *tx, player_id).await?;
    let val = crafting_repo::toggle_auto_sell(&mut *tx, player_id).await?;
    tx.commit().await?;
    Ok(val)
}

pub async fn start_craft(
    pool: &PgPool,
    player_id: i64,
    recipe_id: &str,
    duration_pct: i64,
) -> Result<CraftRunView, CraftError> {
    let recipe = find_recipe(recipe_id).ok_or(CraftError::UnknownRecipe)?;
    let mut tx = pool.begin().await?;
    if !player_repo::lock(&mut *tx, player_id).await? {
        return Err(CraftError::NotRegistered);
    }
    let rep = crafting_repo::ensure_shop_rep(&mut *tx, player_id).await?;
    if crafting_repo::count_running_crafts(&mut *tx, player_id).await? >= MAX_CONCURRENT_CRAFTS {
        return Err(CraftError::TooManyCrafts(MAX_CONCURRENT_CRAFTS));
    }
    for (item_id, need) in recipe.inputs {
        let have = inventory_repo::quantity_of(&mut *tx, player_id, item_id).await?;
        if have < *need {
            let name = find_item(item_id).map(|i| i.name).unwrap_or(item_id);
            return Err(CraftError::MissingIngredient { name, need: *need, have });
        }
    }
    for (item_id, need) in recipe.inputs {
        inventory_repo::remove_item(&mut *tx, player_id, item_id, *need).await?;
    }
    let secs = (recipe.duration_secs * duration_pct.max(1) / 100).max(1);
    let (id, ends_at) = crafting_repo::insert_craft(
        &mut *tx,
        player_id,
        recipe.id,
        recipe.output_qty,
        rep.auto_sell,
        secs,
    )
    .await?;
    scheduler::enqueue(&mut *tx, JOB_CRAFT_COMPLETE, id, ends_at).await?;
    tx.commit().await?;
    Ok(CraftRunView {
        id,
        recipe,
        quantity: recipe.output_qty,
        auto_sell: rep.auto_sell,
        ends_at,
    })
}

/// Dipanggil worker saat durasi crafting habis. Idempoten.
pub async fn complete_craft(conn: &mut PgConnection, craft_id: i64) -> anyhow::Result<()> {
    let Some(job) = crafting_repo::lock_craft(conn, craft_id).await? else {
        return Ok(());
    };
    if job.status != "running" {
        return Ok(());
    }
    let recipe = find_recipe(&job.recipe_id)
        .ok_or_else(|| anyhow::anyhow!("resep tidak dikenal: {}", job.recipe_id))?;
    if let Some(item) = find_item(recipe.output_item_id) {
        if job.auto_sell {
            let unit = market::current_npc_price_conn(conn, item).await?;
            let total = unit * Decimal::from(job.quantity);
            let reason = format!("auto_sell_craft:{}x{}", item.id, job.quantity);
            let key = format!("craft_autosell:{}", job.id);
            let _ = wallet_repo::apply(conn, job.player_id, total, &reason, &key).await?;
        } else {
            let cap = inventory::slot_capacity_conn(conn, job.player_id).await?;
            inventory_repo::add_item(conn, job.player_id, item, job.quantity, cap).await?;
        }
    }
    crafting_repo::mark_craft_completed(conn, craft_id).await?;
    Ok(())
}

pub async fn fulfill_order(
    pool: &PgPool,
    player_id: i64,
    order_id: i64,
) -> Result<OrderFulfillSummary, CraftError> {
    let mut tx = pool.begin().await?;
    if !player_repo::lock(&mut *tx, player_id).await? {
        return Err(CraftError::NotRegistered);
    }
    let Some(order) = crafting_repo::lock_order(&mut *tx, order_id).await? else {
        return Err(CraftError::OrderNotFound);
    };
    if order.player_id != player_id || order.status != "active" {
        return Err(CraftError::OrderNotFound);
    }
    let item = find_item(&order.item_id).ok_or(CraftError::OrderNotFound)?;
    let have = inventory_repo::quantity_of(&mut *tx, player_id, item.id).await?;
    if have < order.quantity {
        return Err(CraftError::InsufficientOrderItem {
            name: item.name,
            need: order.quantity,
            have,
        });
    }

    let rep = crafting_repo::ensure_shop_rep(&mut *tx, player_id).await?;
    let total_reward = apply_rep_bonus(order.reward_bac, rep.level);
    let reason = format!("npc_order:{}x{}", item.id, order.quantity);
    let key = format!("order:{}", order.id);
    if let ApplyOutcome::Duplicate =
        wallet_repo::apply(&mut *tx, player_id, total_reward, &reason, &key).await?
    {
        return Err(CraftError::OrderNotFound);
    }

    inventory_repo::remove_item(&mut *tx, player_id, item.id, order.quantity).await?;
    let (new_lvl, new_xp) = apply_rep_xp(rep.level, rep.xp, order.rep_xp);
    crafting_repo::update_shop_rep(&mut *tx, player_id, new_lvl, new_xp).await?;
    crafting_repo::mark_order_fulfilled(&mut *tx, order.id).await?;
    ensure_orders_conn(&mut *tx, player_id).await?;
    tx.commit().await?;

    Ok(OrderFulfillSummary {
        item,
        quantity: order.quantity,
        reward_bac: total_reward,
        rep_xp_gain: order.rep_xp,
        old_rep_level: rep.level,
        new_rep_level: new_lvl,
    })
}
