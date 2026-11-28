use rust_decimal::Decimal;
use std::collections::HashMap;

pub const MAX_CONCURRENT_CRAFTS: i64 = 2;
pub const MAX_ACTIVE_ORDERS: i64 = 3;
pub const MAX_CRAFT_BATCH: i64 = 1_000;
pub const ORDER_REFRESH_COOLDOWN_SECS: i64 = 300;
pub const ORDER_PLAYER_XP_PER_UNIT: i64 = 20;

pub fn craft_xp_per_unit(tier: RecipeTier) -> i64 {
    match tier {
        RecipeTier::Processed => 10,
        RecipeTier::Finished => 20,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeTier {
    Processed,
    Finished,
}

impl RecipeTier {
    pub fn name(&self) -> &'static str {
        match self {
            RecipeTier::Processed => "Processed",
            RecipeTier::Finished => "Finished",
        }
    }
}

#[derive(Debug)]
pub struct RecipeDef {
    pub id: &'static str,
    pub name: &'static str,
    pub tier: RecipeTier,
    pub output_item_id: &'static str,
    pub output_qty: i64,
    pub inputs: &'static [(&'static str, i64)],
    pub duration_secs: i64,
}

pub const RECIPES: &[RecipeDef] = &[
    RecipeDef {
        id: "flour",
        name: "Wheat Flour",
        tier: RecipeTier::Processed,
        output_item_id: "flour",
        output_qty: 1,
        inputs: &[("wheat", 2)],
        duration_secs: 60,
    },
    RecipeDef {
        id: "plank",
        name: "Wood Plank",
        tier: RecipeTier::Processed,
        output_item_id: "plank",
        output_qty: 1,
        inputs: &[("wood", 2)],
        duration_secs: 60,
    },
    RecipeDef {
        id: "herb_extract",
        name: "Herb Extract",
        tier: RecipeTier::Processed,
        output_item_id: "herb_extract",
        output_qty: 1,
        inputs: &[("wild_herb", 2), ("berries", 1)],
        duration_secs: 90,
    },
    RecipeDef {
        id: "bread",
        name: "Village Bread",
        tier: RecipeTier::Finished,
        output_item_id: "bread",
        output_qty: 1,
        inputs: &[("flour", 2)],
        duration_secs: 120,
    },
    RecipeDef {
        id: "wooden_crate",
        name: "Wooden Crate",
        tier: RecipeTier::Finished,
        output_item_id: "wooden_crate",
        output_qty: 1,
        inputs: &[("plank", 2), ("wood", 1)],
        duration_secs: 120,
    },
    RecipeDef {
        id: "healing_tonic",
        name: "Herbal Tonic",
        tier: RecipeTier::Finished,
        output_item_id: "healing_tonic",
        output_qty: 1,
        inputs: &[("herb_extract", 1), ("berries", 2)],
        duration_secs: 150,
    },
];

pub fn find_recipe(id: &str) -> Option<&'static RecipeDef> {
    RECIPES.iter().find(|r| r.id == id)
}

pub fn rep_xp_to_next(level: i32) -> i64 {
    50 * (level.max(1) as i64)
}

/// Bonus reward pesanan NPC dari reputasi toko (+3% per level di atas 1).
pub fn rep_bonus_pct(level: i32) -> i64 {
    (level.max(1) - 1) as i64 * 3
}

pub fn apply_rep_bonus(base: Decimal, rep_level: i32) -> Decimal {
    let mult = Decimal::from(100 + rep_bonus_pct(rep_level)) / Decimal::from(100);
    (base * mult).round_dp(2)
}

pub fn apply_rep_xp(mut level: i32, mut xp: i64, gain: i64) -> (i32, i64) {
    let mut pool = xp + gain.max(0);
    loop {
        let need = rep_xp_to_next(level);
        if pool >= need {
            pool -= need;
            level += 1;
        } else {
            xp = pool;
            break;
        }
    }
    (level, xp)
}

pub fn batch_duration_secs(base_secs: i64, qty: i64) -> i64 {
    let q = qty.max(1);
    if q == 1 {
        base_secs
    } else {
        base_secs + (q - 1) * (base_secs / 4).max(5)
    }
}

pub fn max_craftable_batches(recipe: &RecipeDef, stock: &HashMap<&str, i64>) -> i64 {
    recipe
        .inputs
        .iter()
        .map(|(id, need)| stock.get(id).copied().unwrap_or(0).max(0) / (*need).max(1))
        .min()
        .unwrap_or(0)
        .clamp(0, MAX_CRAFT_BATCH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::resources::find_item;

    #[test]
    fn all_recipes_reference_valid_items() {
        for r in RECIPES {
            assert!(find_item(r.output_item_id).is_some(), "missing output {}", r.output_item_id);
            assert!(r.output_qty > 0);
            for (in_id, qty) in r.inputs {
                assert!(find_item(in_id).is_some(), "missing input {in_id}");
                assert!(*qty > 0);
            }
        }
    }

    #[test]
    fn rep_xp_and_bonus_scale() {
        assert_eq!(apply_rep_xp(1, 0, 50), (2, 0));
        assert_eq!(apply_rep_xp(1, 10, 55), (2, 15));
        assert_eq!(apply_rep_bonus(Decimal::new(10_000, 2), 3), Decimal::new(10_600, 2));
    }

    #[test]
    fn batch_duration_scales_with_discount() {
        assert_eq!(batch_duration_secs(60, 1), 60);
        assert_eq!(batch_duration_secs(60, 10), 195);
        assert_eq!(batch_duration_secs(120, 5), 240);
    }

    #[test]
    fn max_craftable_batches_respects_bottleneck_and_cap() {
        let crate_recipe = find_recipe("wooden_crate").unwrap(); // plank x2, wood x1
        let mut stock = HashMap::from([("plank", 11_i64), ("wood", 10_i64)]);
        assert_eq!(max_craftable_batches(crate_recipe, &stock), 5);

        stock.insert("plank", 5_000);
        stock.insert("wood", 5_000);
        assert_eq!(max_craftable_batches(crate_recipe, &stock), MAX_CRAFT_BATCH);

        stock.insert("wood", 0);
        assert_eq!(max_craftable_batches(crate_recipe, &stock), 0);
    }
}
