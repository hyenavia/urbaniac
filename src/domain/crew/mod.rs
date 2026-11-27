use crate::domain::activities::ActivityKind;
use crate::domain::resources::Rarity;
use rand::Rng;
use rust_decimal::Decimal;

pub const MAX_CREW_SLOTS: i64 = 4;
pub const MAX_CREW_LEVEL: i32 = 10;
pub const RECRUIT_COST_CENTS: i64 = 10_000; // 100.00 Bac

pub fn recruit_cost() -> Decimal {
    Decimal::new(RECRUIT_COST_CENTS, 2)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrewBonus {
    pub efficiency_pct: i64,
    pub yield_pct: i64,
    pub luck_pct: i64,
}

impl CrewBonus {
    pub const ZERO: CrewBonus = CrewBonus {
        efficiency_pct: 0,
        yield_pct: 0,
        luck_pct: 0,
    };
}

pub fn parse_rarity(s: &str) -> Rarity {
    match s {
        "uncommon" => Rarity::Uncommon,
        "rare" => Rarity::Rare,
        "epic" => Rarity::Epic,
        _ => Rarity::Common,
    }
}

pub fn rarity_str(r: Rarity) -> &'static str {
    match r {
        Rarity::Common => "common",
        Rarity::Uncommon => "uncommon",
        Rarity::Rare => "rare",
        Rarity::Epic | Rarity::Legendary => "epic",
    }
}

pub fn crew_bonus(rarity: Rarity, level: i32, specialty_match: bool) -> CrewBonus {
    let base = match rarity {
        Rarity::Common => 5,
        Rarity::Uncommon => 10,
        Rarity::Rare => 15,
        Rarity::Epic | Rarity::Legendary => 22,
    };
    let lvl_bonus = (level.max(1) - 1) as i64 * 3;
    let total = base + lvl_bonus;
    let effective = if specialty_match { total } else { (total / 2).max(1) };
    CrewBonus {
        efficiency_pct: effective,
        yield_pct: effective,
        luck_pct: effective,
    }
}

pub fn upgrade_cost(level: i32, rarity: Rarity) -> Decimal {
    let mult = match rarity {
        Rarity::Common => 1,
        Rarity::Uncommon => 2,
        Rarity::Rare => 3,
        Rarity::Epic | Rarity::Legendary => 4,
    };
    Decimal::new(5_000 * (level.max(1) as i64) * mult, 2)
}

pub fn apply_efficiency_duration(base_secs: i64, efficiency_pct: i64) -> i64 {
    ((base_secs * 100) / (100 + efficiency_pct.max(0))).max(1)
}

const NAMES: &[&str] = &[
    "Budi", "Sari", "Raka", "Dewi", "Bagas", "Laras", "Tama", "Nadia", "Fajar", "Putri",
];

pub fn roll_recruit<R: Rng>(rng: &mut R) -> (&'static str, Rarity, ActivityKind) {
    let name = NAMES[rng.gen_range(0..NAMES.len())];
    let r_roll = rng.gen_range(0..100);
    let rarity = if r_roll < 55 {
        Rarity::Common
    } else if r_roll < 82 {
        Rarity::Uncommon
    } else if r_roll < 95 {
        Rarity::Rare
    } else {
        Rarity::Epic
    };
    let spec = ActivityKind::ALL[rng.gen_range(0..ActivityKind::ALL.len())];
    (name, rarity, spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn efficiency_reduces_duration_and_specialty_boosts_bonus() {
        assert_eq!(apply_efficiency_duration(600, 0), 600);
        assert_eq!(apply_efficiency_duration(600, 20), 500);
        let match_b = crew_bonus(Rarity::Rare, 2, true);
        let off_b = crew_bonus(Rarity::Rare, 2, false);
        assert!(match_b.yield_pct > off_b.yield_pct);
    }
}
