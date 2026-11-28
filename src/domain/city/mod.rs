use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CityStage {
    Kampung,
    Desa,
    KotaKecil,
    KotaDagang,
    Metropolis,
}

impl CityStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            CityStage::Kampung => "kampung",
            CityStage::Desa => "desa",
            CityStage::KotaKecil => "kota_kecil",
            CityStage::KotaDagang => "kota_dagang",
            CityStage::Metropolis => "metropolis",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "kampung" => Some(CityStage::Kampung),
            "desa" => Some(CityStage::Desa),
            "kota_kecil" => Some(CityStage::KotaKecil),
            "kota_dagang" => Some(CityStage::KotaDagang),
            "metropolis" => Some(CityStage::Metropolis),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            CityStage::Kampung => "Hamlet",
            CityStage::Desa => "Village",
            CityStage::KotaKecil => "Town",
            CityStage::KotaDagang => "Trade City",
            CityStage::Metropolis => "Metropolis",
        }
    }

    pub fn era(&self) -> &'static str {
        match self {
            CityStage::Kampung | CityStage::Desa => "Village Era",
            CityStage::KotaKecil => "Town Era",
            CityStage::KotaDagang => "City Era",
            CityStage::Metropolis => "Metropolis Era",
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            CityStage::Kampung => Some(CityStage::Desa),
            CityStage::Desa => Some(CityStage::KotaKecil),
            CityStage::KotaKecil => Some(CityStage::KotaDagang),
            CityStage::KotaDagang => Some(CityStage::Metropolis),
            CityStage::Metropolis => None,
        }
    }
}

/// Tambahan slot inventory per level Lumbung. Nilai awal sementara.
pub const GRANARY_SLOTS_PER_LEVEL: i64 = 4;
pub const MAX_GRANARY_LEVEL: i32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectId {
    Well,
    Granary,
    TownHall,
    MarketHall,
    CityHall,
    TradePort,
    MetropolisTower,
}

#[derive(Debug)]
pub struct ProjectDef {
    pub id: ProjectId,
    pub key: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub min_stage: CityStage,
    pub max_level: i32,
    /// Biaya Bac dasar dalam sen (5000 = 50.00 Bac).
    pub cost_bac_cents: i64,
    /// Resource yang dibutuhkan: (item_id, jumlah).
    pub cost_items: &'static [(&'static str, i64)],
    pub duration_secs: i64,
}

impl ProjectId {
    pub const ALL: [ProjectId; 7] = [
        ProjectId::Well,
        ProjectId::Granary,
        ProjectId::TownHall,
        ProjectId::MarketHall,
        ProjectId::CityHall,
        ProjectId::TradePort,
        ProjectId::MetropolisTower,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectId::Well => "well",
            ProjectId::Granary => "granary",
            ProjectId::TownHall => "town_hall",
            ProjectId::MarketHall => "market_hall",
            ProjectId::CityHall => "city_hall",
            ProjectId::TradePort => "trade_port",
            ProjectId::MetropolisTower => "metropolis_tower",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "well" => Some(ProjectId::Well),
            "granary" => Some(ProjectId::Granary),
            "town_hall" => Some(ProjectId::TownHall),
            "market_hall" => Some(ProjectId::MarketHall),
            "city_hall" => Some(ProjectId::CityHall),
            "trade_port" => Some(ProjectId::TradePort),
            "metropolis_tower" => Some(ProjectId::MetropolisTower),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        def(*self).name
    }
}

pub const WELL: ProjectDef = ProjectDef {
    id: ProjectId::Well,
    key: "well",
    name: "Well",
    description: "+10% Farming & Gathering yield. Prerequisite for Village Hall.",
    min_stage: CityStage::Kampung,
    max_level: 1,
    cost_bac_cents: 5_000,
    cost_items: &[("wood", 10)],
    duration_secs: 300,
};

pub const GRANARY: ProjectDef = ProjectDef {
    id: ProjectId::Granary,
    key: "granary",
    name: "Granary",
    description: "+4 bag slots & +3% all activity yield per level (max Lv 5).",
    min_stage: CityStage::Kampung,
    max_level: MAX_GRANARY_LEVEL,
    cost_bac_cents: 8_000,
    cost_items: &[("wood", 15), ("wheat", 10)],
    duration_secs: 480,
};

pub const TOWN_HALL: ProjectDef = ProjectDef {
    id: ProjectId::TownHall,
    key: "town_hall",
    name: "Village Hall",
    description: "Advances city to Village (unlocks Fishing, 3 Activity Slots, Lv 20 cap; needs Well & Granary).",
    min_stage: CityStage::Kampung,
    max_level: 1,
    cost_bac_cents: 15_000,
    cost_items: &[("wood", 25), ("potato", 10)],
    duration_secs: 600,
};

pub const MARKET_HALL: ProjectDef = ProjectDef {
    id: ProjectId::MarketHall,
    key: "market_hall",
    name: "Market Hall",
    description: "+5% direct NPC sell price. Prerequisite for Town Hall.",
    min_stage: CityStage::Desa,
    max_level: 1,
    cost_bac_cents: 20_000,
    cost_items: &[("plank", 10), ("bread", 5)],
    duration_secs: 600,
};

pub const CITY_HALL: ProjectDef = ProjectDef {
    id: ProjectId::CityHall,
    key: "city_hall",
    name: "Town Hall",
    description: "Advances city to Town (unlocks Mining, 4 Activity Slots, Lv 30 cap; needs Market Hall).",
    min_stage: CityStage::Desa,
    max_level: 1,
    cost_bac_cents: 35_000,
    cost_items: &[("wooden_crate", 5), ("river_fish", 10)],
    duration_secs: 900,
};

pub const TRADE_PORT: ProjectDef = ProjectDef {
    id: ProjectId::TradePort,
    key: "trade_port",
    name: "Trade Port",
    description: "Advances city to Trade City (Lv 40 cap, 3 Craft Slots, +15% NPC Order rewards).",
    min_stage: CityStage::KotaKecil,
    max_level: 1,
    cost_bac_cents: 60_000,
    cost_items: &[("plank", 20), ("iron_ore", 10)],
    duration_secs: 1200,
};

pub const METROPOLIS_TOWER: ProjectDef = ProjectDef {
    id: ProjectId::MetropolisTower,
    key: "metropolis_tower",
    name: "Metropolis Tower",
    description: "Advances city to Metropolis (Lv 50 cap, +15% Activity & Craft speed).",
    min_stage: CityStage::KotaDagang,
    max_level: 1,
    cost_bac_cents: 120_000,
    cost_items: &[("amethyst", 5), ("healing_tonic", 5)],
    duration_secs: 1800,
};

pub fn def(id: ProjectId) -> &'static ProjectDef {
    match id {
        ProjectId::Well => &WELL,
        ProjectId::Granary => &GRANARY,
        ProjectId::TownHall => &TOWN_HALL,
        ProjectId::MarketHall => &MARKET_HALL,
        ProjectId::CityHall => &CITY_HALL,
        ProjectId::TradePort => &TRADE_PORT,
        ProjectId::MetropolisTower => &METROPOLIS_TOWER,
    }
}

pub fn project_cost_bac_for_level(p: &ProjectDef, next_level: i32) -> Decimal {
    Decimal::new(p.cost_bac_cents * (next_level.max(1) as i64), 2)
}

pub fn project_cost_bac(p: &ProjectDef) -> Decimal {
    project_cost_bac_for_level(p, 1)
}

pub fn inventory_capacity_with_granary(base: i64, granary_level: i32) -> i64 {
    base + (granary_level.max(0) as i64) * GRANARY_SLOTS_PER_LEVEL
}

pub const CITY_ECONOMY_TICK_SECS: i64 = 1800; // 30 menit (diskalakan ACTIVITY_DURATION_PCT)
pub const MIN_TAX_RATE: i32 = 0;
pub const MAX_TAX_RATE: i32 = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CityEconomyOutcome {
    pub tax_income: Decimal,
    pub maintenance: Decimal,
    pub crew_salary: Decimal,
    pub treasury_delta: Decimal,
    pub new_treasury: Decimal,
    pub new_welfare: i32,
    pub new_effectiveness_pct: i32,
    pub shortfall: bool,
}

pub fn effectiveness_from_welfare(welfare: i32) -> i32 {
    50 + (welfare.clamp(0, 100) * 65 / 100)
}

pub fn yield_bonus_from_effectiveness(effectiveness_pct: i32) -> i64 {
    ((effectiveness_pct - 100) / 2) as i64
}

pub fn local_modifier_from_city(stage: CityStage, welfare: i32) -> Decimal {
    let base = match stage {
        CityStage::Kampung => Decimal::new(10_000, 4),
        CityStage::Desa => Decimal::new(10_200, 4),
        CityStage::KotaKecil => Decimal::new(10_500, 4),
        CityStage::KotaDagang => Decimal::new(11_000, 4),
        CityStage::Metropolis => Decimal::new(11_500, 4),
    };
    let penalty = if welfare < 40 {
        Decimal::new(500, 4)
    } else {
        Decimal::ZERO
    };
    (base - penalty)
        .round_dp(4)
        .clamp(Decimal::new(8_000, 4), Decimal::new(13_000, 4))
}

/// Menghitung satu siklus pajak warga, maintenance fasilitas, gaji kru, dan welfare.
/// Treasury tidak pernah negatif dan tidak ada denda; kekurangan hanya menurunkan welfare & efektivitas bertahap.
pub fn settle_city_economy_tick(
    treasury: Decimal,
    stage: CityStage,
    facility_count: i64,
    crew_count: i64,
    tax_rate_pct: i32,
    welfare: i32,
) -> CityEconomyOutcome {
    let stage_base = match stage {
        CityStage::Kampung => Decimal::new(1_500, 2),
        CityStage::Desa => Decimal::new(3_500, 2),
        CityStage::KotaKecil => Decimal::new(7_500, 2),
        CityStage::KotaDagang => Decimal::new(14_000, 2),
        CityStage::Metropolis => Decimal::new(26_000, 2),
    };
    let base_tax = stage_base + Decimal::new(800 * facility_count.max(0), 2);
    let tax_pct = tax_rate_pct.clamp(MIN_TAX_RATE, MAX_TAX_RATE);
    let tax_income = (base_tax * Decimal::from(tax_pct) / Decimal::from(10)
        * Decimal::from(welfare.clamp(20, 100))
        / Decimal::from(100))
    .round_dp(2);

    let maintenance = Decimal::new(500 * facility_count.max(0), 2);
    let crew_salary = Decimal::new(800 * crew_count.max(0), 2);
    let available = (treasury + tax_income).round_dp(2);
    let expense = maintenance + crew_salary;

    let (new_treasury, shortfall, new_welfare) = if available >= expense {
        let next_w = if tax_pct <= 5 {
            (welfare + 10).clamp(0, 100)
        } else if tax_pct <= 10 {
            (welfare + 4).clamp(0, 100)
        } else if tax_pct <= 15 {
            if welfare > 80 {
                (welfare - 3).max(80)
            } else {
                (welfare + 2).min(80)
            }
        } else if welfare > 55 {
            (welfare - 5).max(55)
        } else {
            welfare.clamp(0, 100)
        };
        ((available - expense).round_dp(2), false, next_w)
    } else {
        (Decimal::ZERO, true, (welfare - 10).clamp(0, 100))
    };

    let treasury_delta = (new_treasury - treasury).round_dp(2);
    let new_effectiveness_pct = effectiveness_from_welfare(new_welfare);

    CityEconomyOutcome {
        tax_income,
        maintenance,
        crew_salary,
        treasury_delta,
        new_treasury,
        new_welfare,
        new_effectiveness_pct,
        shortfall,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::resources::{find_item, BASE_SLOT_CAPACITY};

    #[test]
    fn stage_roundtrip() {
        for s in [
            CityStage::Kampung,
            CityStage::Desa,
            CityStage::KotaKecil,
            CityStage::KotaDagang,
            CityStage::Metropolis,
        ] {
            assert_eq!(CityStage::parse(s.as_str()), Some(s));
        }
        assert_eq!(CityStage::parse("unknown"), None);
    }

    #[test]
    fn project_roundtrip_and_items_exist() {
        for p in ProjectId::ALL {
            assert_eq!(ProjectId::parse(p.as_str()), Some(p));
            for (item_id, qty) in def(p).cost_items {
                assert!(find_item(item_id).is_some(), "missing item {item_id}");
                assert!(*qty > 0);
            }
        }
    }

    #[test]
    fn granary_increases_inventory_capacity() {
        assert_eq!(inventory_capacity_with_granary(BASE_SLOT_CAPACITY, 0), 12);
        assert_eq!(inventory_capacity_with_granary(BASE_SLOT_CAPACITY, 1), 16);
        assert_eq!(inventory_capacity_with_granary(BASE_SLOT_CAPACITY, 5), 32);
    }

    #[test]
    fn city_economy_never_negative_and_low_treasury_reduces_welfare() {
        let ok = settle_city_economy_tick(
            Decimal::new(10_000, 2),
            CityStage::Desa,
            2,
            1,
            10,
            80,
        );
        assert!(!ok.shortfall);
        assert!(ok.new_treasury >= Decimal::ZERO);
        assert!(ok.new_welfare > 80);

        // Treasury 0 dengan pajak 0% dan beban biaya -> tidak negatif, welfare turun bertahap
        let empty = settle_city_economy_tick(
            Decimal::ZERO,
            CityStage::Kampung,
            3,
            2,
            0,
            70,
        );
        assert!(empty.shortfall);
        assert_eq!(empty.new_treasury, Decimal::ZERO);
        assert!(empty.new_welfare < 70);
        assert!(empty.new_effectiveness_pct < effectiveness_from_welfare(70));
    }

    #[test]
    fn high_tax_trades_welfare_for_income() {
        let t15 = settle_city_economy_tick(
            Decimal::new(10_000, 2),
            CityStage::Desa,
            2,
            1,
            15,
            100,
        );
        assert!(!t15.shortfall);
        assert_eq!(t15.new_welfare, 97);

        let t10 = settle_city_economy_tick(
            Decimal::new(10_000, 2),
            CityStage::Desa,
            2,
            1,
            10,
            90,
        );
        assert!(t10.new_welfare > 90);
        assert!(t15.tax_income > t10.tax_income);
    }
}
