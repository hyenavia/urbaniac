use crate::domain::city::CityStage;
use rand::Rng;
use std::collections::BTreeMap;

/// Jumlah Activity Slot dasar. Crew bukan pengganti slot. Nilai awal sementara.
pub const BASE_ACTIVITY_SLOTS: i64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityKind {
    Farming,
    Gathering,
    Fishing,
    Mining,
}

impl ActivityKind {
    pub const ALL: [ActivityKind; 4] = [
        ActivityKind::Farming,
        ActivityKind::Gathering,
        ActivityKind::Fishing,
        ActivityKind::Mining,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityKind::Farming => "farming",
            ActivityKind::Gathering => "gathering",
            ActivityKind::Fishing => "fishing",
            ActivityKind::Mining => "mining",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ActivityKind::Farming => "Farming",
            ActivityKind::Gathering => "Gathering",
            ActivityKind::Fishing => "Fishing",
            ActivityKind::Mining => "Mining",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|k| k.as_str() == s)
    }

    /// Stage minimum kota agar aktivitas terbuka.
    pub fn min_stage(&self) -> CityStage {
        match self {
            ActivityKind::Farming | ActivityKind::Gathering => CityStage::Kampung,
            ActivityKind::Fishing => CityStage::Desa,
            ActivityKind::Mining => CityStage::KotaKecil,
        }
    }
}

#[derive(Debug)]
pub struct DropEntry {
    pub item_id: &'static str,
    pub min: i64,
    pub max: i64,
    pub weight: u32,
}

#[derive(Debug)]
pub struct ActivityDef {
    pub kind: ActivityKind,
    pub duration_secs: i64,
    pub xp_reward: i64,
    pub rolls: u32,
    pub drops: &'static [DropEntry],
}

pub static FARMING: ActivityDef = ActivityDef {
    kind: ActivityKind::Farming,
    duration_secs: 600,
    xp_reward: 10,
    rolls: 3,
    drops: &[
        DropEntry { item_id: "wheat", min: 2, max: 4, weight: 70 },
        DropEntry { item_id: "potato", min: 1, max: 3, weight: 30 },
    ],
};

pub static GATHERING: ActivityDef = ActivityDef {
    kind: ActivityKind::Gathering,
    duration_secs: 300,
    xp_reward: 10,
    rolls: 3,
    drops: &[
        DropEntry { item_id: "wood", min: 2, max: 4, weight: 50 },
        DropEntry { item_id: "berries", min: 2, max: 4, weight: 35 },
        DropEntry { item_id: "wild_herb", min: 1, max: 2, weight: 15 },
    ],
};

pub static FISHING: ActivityDef = ActivityDef {
    kind: ActivityKind::Fishing,
    duration_secs: 480,
    xp_reward: 15,
    rolls: 3,
    drops: &[
        DropEntry { item_id: "river_fish", min: 2, max: 4, weight: 65 },
        DropEntry { item_id: "silver_carp", min: 1, max: 2, weight: 28 },
        DropEntry { item_id: "golden_eel", min: 1, max: 1, weight: 7 },
    ],
};

pub static MINING: ActivityDef = ActivityDef {
    kind: ActivityKind::Mining,
    duration_secs: 720,
    xp_reward: 20,
    rolls: 3,
    drops: &[
        DropEntry { item_id: "stone", min: 2, max: 5, weight: 60 },
        DropEntry { item_id: "iron_ore", min: 1, max: 3, weight: 28 },
        DropEntry { item_id: "amethyst", min: 1, max: 1, weight: 10 },
        DropEntry { item_id: "star_crystal", min: 1, max: 1, weight: 2 },
    ],
};

pub fn def(kind: ActivityKind) -> &'static ActivityDef {
    match kind {
        ActivityKind::Farming => &FARMING,
        ActivityKind::Gathering => &GATHERING,
        ActivityKind::Fishing => &FISHING,
        ActivityKind::Mining => &MINING,
    }
}

/// Bonus yield dari level aktivitas: +2% per level di atas 1. Nilai awal sementara.
pub fn yield_bonus_pct(level: i32) -> i64 {
    2 * (level.max(1) - 1) as i64
}

/// Mengundi reward saat aktivitas selesai.
pub fn roll_rewards<R: Rng>(def: &ActivityDef, level: i32, rng: &mut R) -> Vec<(&'static str, i64)> {
    roll_rewards_with_bonus(def, level, 0, 0, rng)
}

/// Mengundi reward dengan tambahan bonus Yield dan Luck (dari Kru & Kota).
pub fn roll_rewards_with_bonus<R: Rng>(
    def: &ActivityDef,
    level: i32,
    extra_yield_pct: i64,
    luck_pct: i64,
    rng: &mut R,
) -> Vec<(&'static str, i64)> {
    let total: u32 = def.drops.iter().map(|d| d.weight).sum();
    let bonus = (yield_bonus_pct(level) + extra_yield_pct).max(-50);
    let mut out: BTreeMap<&'static str, i64> = BTreeMap::new();
    if total == 0 {
        return Vec::new();
    }
    let extra_roll = if luck_pct > 0 && rng.gen_range(0..100) < luck_pct.min(100) {
        1
    } else {
        0
    };
    for _ in 0..(def.rolls + extra_roll) {
        let mut pick = rng.gen_range(0..total);
        for d in def.drops {
            if pick < d.weight {
                let base = rng.gen_range(d.min..=d.max);
                let qty = ((base * (100 + bonus) + 50) / 100).max(1);
                *out.entry(d.item_id).or_insert(0) += qty;
                break;
            }
            pick -= d.weight;
        }
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::resources::find_item;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn parse_roundtrip() {
        for k in ActivityKind::ALL {
            assert_eq!(ActivityKind::parse(k.as_str()), Some(k));
        }
        assert_eq!(ActivityKind::parse("x"), None);
    }

    #[test]
    fn all_drops_exist_in_catalog() {
        for k in ActivityKind::ALL {
            for d in def(k).drops {
                assert!(find_item(d.item_id).is_some(), "{} tidak ada", d.item_id);
                assert!(d.min >= 1 && d.min <= d.max);
            }
        }
    }

    #[test]
    fn rolls_stay_in_bounds_and_are_deterministic() {
        let d = def(ActivityKind::Farming);
        let a = roll_rewards(d, 1, &mut StdRng::seed_from_u64(7));
        let b = roll_rewards(d, 1, &mut StdRng::seed_from_u64(7));
        assert_eq!(a, b);
        let total: i64 = a.iter().map(|(_, q)| q).sum();
        assert!(total >= d.rolls as i64 && total <= 4 * d.rolls as i64);
    }

    #[test]
    fn level_bonus_never_reduces_yield() {
        let d = def(ActivityKind::Gathering);
        for seed in 0..20 {
            let lo: i64 = roll_rewards(d, 1, &mut StdRng::seed_from_u64(seed)).iter().map(|x| x.1).sum();
            let hi: i64 = roll_rewards(d, 10, &mut StdRng::seed_from_u64(seed)).iter().map(|x| x.1).sum();
            assert!(hi >= lo);
        }
    }
}
