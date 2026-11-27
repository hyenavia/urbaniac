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
            CityStage::Kampung => "Kampung",
            CityStage::Desa => "Desa",
            CityStage::KotaKecil => "Kota Kecil",
            CityStage::KotaDagang => "Kota Dagang",
            CityStage::Metropolis => "Metropolis",
        }
    }
}

/// Tambahan slot inventory per level Lumbung. Nilai awal sementara.
pub const GRANARY_SLOTS_PER_LEVEL: i64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectId {
    Well,
    Granary,
    TownHall,
}

#[derive(Debug)]
pub struct ProjectDef {
    pub id: ProjectId,
    pub key: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// Biaya Bac dalam sen (5000 = 50.00 Bac).
    pub cost_bac_cents: i64,
    /// Resource yang dibutuhkan: (item_id, jumlah).
    pub cost_items: &'static [(&'static str, i64)],
    pub duration_secs: i64,
}

impl ProjectId {
    pub const ALL: [ProjectId; 3] = [ProjectId::Well, ProjectId::Granary, ProjectId::TownHall];

    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectId::Well => "well",
            ProjectId::Granary => "granary",
            ProjectId::TownHall => "town_hall",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "well" => Some(ProjectId::Well),
            "granary" => Some(ProjectId::Granary),
            "town_hall" => Some(ProjectId::TownHall),
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
    name: "Sumur",
    description: "Sumber air bersih Kampung. Prasyarat Balai Kampung.",
    cost_bac_cents: 5_000,
    cost_items: &[("wood", 10)],
    duration_secs: 300,
};

pub const GRANARY: ProjectDef = ProjectDef {
    id: ProjectId::Granary,
    key: "granary",
    name: "Lumbung",
    description: "Menambah +4 slot kapasitas inventory.",
    cost_bac_cents: 8_000,
    cost_items: &[("wood", 15), ("wheat", 10)],
    duration_secs: 480,
};

pub const TOWN_HALL: ProjectDef = ProjectDef {
    id: ProjectId::TownHall,
    key: "town_hall",
    name: "Balai Kampung",
    description: "Menaikkan stage kota dari Kampung menjadi Desa (butuh Sumur & Lumbung).",
    cost_bac_cents: 15_000,
    cost_items: &[("wood", 25), ("potato", 10)],
    duration_secs: 600,
};

pub fn def(id: ProjectId) -> &'static ProjectDef {
    match id {
        ProjectId::Well => &WELL,
        ProjectId::Granary => &GRANARY,
        ProjectId::TownHall => &TOWN_HALL,
    }
}

pub fn project_cost_bac(p: &ProjectDef) -> Decimal {
    Decimal::new(p.cost_bac_cents, 2)
}

pub fn inventory_capacity_with_granary(base: i64, granary_level: i32) -> i64 {
    base + (granary_level.max(0) as i64) * GRANARY_SLOTS_PER_LEVEL
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
        assert_eq!(inventory_capacity_with_granary(BASE_SLOT_CAPACITY, 2), 20);
    }
}
