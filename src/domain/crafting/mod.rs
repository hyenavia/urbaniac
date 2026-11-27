use rust_decimal::Decimal;

pub const MAX_CONCURRENT_CRAFTS: i64 = 2;
pub const MAX_ACTIVE_ORDERS: i64 = 3;

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
        name: "Tepung Gandum",
        tier: RecipeTier::Processed,
        output_item_id: "flour",
        output_qty: 1,
        inputs: &[("wheat", 2)],
        duration_secs: 60,
    },
    RecipeDef {
        id: "plank",
        name: "Papan Kayu",
        tier: RecipeTier::Processed,
        output_item_id: "plank",
        output_qty: 1,
        inputs: &[("wood", 2)],
        duration_secs: 60,
    },
    RecipeDef {
        id: "herb_extract",
        name: "Ekstrak Herba",
        tier: RecipeTier::Processed,
        output_item_id: "herb_extract",
        output_qty: 1,
        inputs: &[("wild_herb", 2), ("berries", 1)],
        duration_secs: 90,
    },
    RecipeDef {
        id: "bread",
        name: "Roti Kampung",
        tier: RecipeTier::Finished,
        output_item_id: "bread",
        output_qty: 1,
        inputs: &[("flour", 2)],
        duration_secs: 120,
    },
    RecipeDef {
        id: "wooden_crate",
        name: "Peti Kayu",
        tier: RecipeTier::Finished,
        output_item_id: "wooden_crate",
        output_qty: 1,
        inputs: &[("plank", 2), ("wood", 1)],
        duration_secs: 120,
    },
    RecipeDef {
        id: "healing_tonic",
        name: "Tonik Herba",
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
}
