use crate::domain::activities::ActivityKind;
use crate::domain::city::{CityStage, ProjectId};
use crate::domain::crafting::RecipeTier;
use crate::domain::resources::{find_item, ITEMS};
use crate::domain::world::{Season, WorldEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Id,
}

impl Lang {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "id" => Lang::Id,
            _ => Lang::En,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Id => "id",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            Lang::En => Lang::Id,
            Lang::Id => Lang::En,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Id => "Bahasa Indonesia",
        }
    }
}

pub fn item_name(id: &str, lang: Lang) -> &'static str {
    match (id, lang) {
        ("wheat", Lang::En) => "Wheat",
        ("wheat", Lang::Id) => "Gandum",
        ("potato", Lang::En) => "Potato",
        ("potato", Lang::Id) => "Kentang",
        ("wood", Lang::En) => "Wood",
        ("wood", Lang::Id) => "Kayu",
        ("berries", Lang::En) => "Forest Berries",
        ("berries", Lang::Id) => "Buah Hutan",
        ("wild_herb", Lang::En) => "Wild Herb",
        ("wild_herb", Lang::Id) => "Herba Liar",
        ("flour", Lang::En) => "Wheat Flour",
        ("flour", Lang::Id) => "Tepung Gandum",
        ("plank", Lang::En) => "Wood Plank",
        ("plank", Lang::Id) => "Papan Kayu",
        ("herb_extract", Lang::En) => "Herb Extract",
        ("herb_extract", Lang::Id) => "Ekstrak Herba",
        ("bread", Lang::En) => "Village Bread",
        ("bread", Lang::Id) => "Roti Kampung",
        ("wooden_crate", Lang::En) => "Wooden Crate",
        ("wooden_crate", Lang::Id) => "Peti Kayu",
        ("healing_tonic", Lang::En) => "Herbal Tonic",
        ("healing_tonic", Lang::Id) => "Tonik Herba",
        ("river_fish", Lang::En) => "River Fish",
        ("river_fish", Lang::Id) => "Ikan Sungai",
        ("silver_carp", Lang::En) => "Silver Carp",
        ("silver_carp", Lang::Id) => "Gurame Perak",
        ("golden_eel", Lang::En) => "Golden Eel",
        ("golden_eel", Lang::Id) => "Belut Emas",
        ("stone", Lang::En) => "Stone",
        ("stone", Lang::Id) => "Batu",
        ("iron_ore", Lang::En) => "Iron Ore",
        ("iron_ore", Lang::Id) => "Bijih Besi",
        ("amethyst", Lang::En) => "Amethyst",
        ("amethyst", Lang::Id) => "Kecubung",
        ("star_crystal", Lang::En) => "Star Crystal",
        ("star_crystal", Lang::Id) => "Kristal Bintang",
        _ => find_item(id).map(|i| i.name).unwrap_or("?"),
    }
}

/// Translates either an item ID or its Indonesian catalog name into the requested language.
pub fn item_name_by_any(id_or_name: &str, lang: Lang) -> &str {
    if let Some(it) = ITEMS
        .iter()
        .find(|i| i.id == id_or_name || i.name == id_or_name)
    {
        item_name(it.id, lang)
    } else {
        id_or_name
    }
}

pub fn stage_name(stage: CityStage, lang: Lang) -> &'static str {
    match (stage, lang) {
        (CityStage::Kampung, Lang::En) => "Hamlet",
        (CityStage::Kampung, Lang::Id) => "Kampung",
        (CityStage::Desa, Lang::En) => "Village",
        (CityStage::Desa, Lang::Id) => "Desa",
        (CityStage::KotaKecil, Lang::En) => "Town",
        (CityStage::KotaKecil, Lang::Id) => "Kota Kecil",
        (CityStage::KotaDagang, Lang::En) => "Trade City",
        (CityStage::KotaDagang, Lang::Id) => "Kota Dagang",
        (CityStage::Metropolis, _) => "Metropolis",
    }
}

pub fn stage_era(stage: CityStage, lang: Lang) -> &'static str {
    match (stage, lang) {
        (CityStage::Kampung | CityStage::Desa, Lang::En) => "Village Era",
        (CityStage::Kampung | CityStage::Desa, Lang::Id) => "Era Desa",
        (CityStage::KotaKecil, Lang::En) => "Town Era",
        (CityStage::KotaKecil, Lang::Id) => "Era Kota Kecil",
        (CityStage::KotaDagang, Lang::En) => "City Era",
        (CityStage::KotaDagang, Lang::Id) => "Era Kota Dagang",
        (CityStage::Metropolis, Lang::En) => "Metropolis Era",
        (CityStage::Metropolis, Lang::Id) => "Era Metropolis",
    }
}

pub fn project_name(id: ProjectId, lang: Lang) -> &'static str {
    match (id, lang) {
        (ProjectId::Well, Lang::En) => "Well",
        (ProjectId::Well, Lang::Id) => "Sumur",
        (ProjectId::Granary, Lang::En) => "Granary",
        (ProjectId::Granary, Lang::Id) => "Lumbung",
        (ProjectId::TownHall, Lang::En) => "Village Hall",
        (ProjectId::TownHall, Lang::Id) => "Balai Kampung",
        (ProjectId::MarketHall, Lang::En) => "Market Hall",
        (ProjectId::MarketHall, Lang::Id) => "Pasar Desa",
        (ProjectId::CityHall, Lang::En) => "Town Hall",
        (ProjectId::CityHall, Lang::Id) => "Balai Kota Kecil",
        (ProjectId::TradePort, Lang::En) => "Trade Port",
        (ProjectId::TradePort, Lang::Id) => "Pelabuhan Dagang",
        (ProjectId::MetropolisTower, Lang::En) => "Metropolis Tower",
        (ProjectId::MetropolisTower, Lang::Id) => "Menara Metropolis",
    }
}

pub fn project_desc(id: ProjectId, lang: Lang) -> &'static str {
    match (id, lang) {
        (ProjectId::Well, Lang::En) => "Clean water source for your Hamlet. Prerequisite for Village Hall.",
        (ProjectId::Well, Lang::Id) => "Sumber air bersih Kampung. Prasyarat Balai Kampung.",
        (ProjectId::Granary, Lang::En) => "Adds +4 inventory capacity slots per level (max Lv 5).",
        (ProjectId::Granary, Lang::Id) => "Menambah +4 slot kapasitas inventory per level (maks Lv 5).",
        (ProjectId::TownHall, Lang::En) => "Advances your city to Village (unlocks Fishing & Lv 20 cap; requires Well & Granary).",
        (ProjectId::TownHall, Lang::Id) => "Naikkan kota ke Desa (buka Fishing & cap Lv 20; butuh Sumur & Lumbung).",
        (ProjectId::MarketHall, Lang::En) => "Village trade center. Prerequisite for Town Hall.",
        (ProjectId::MarketHall, Lang::Id) => "Pusat niaga Desa. Prasyarat Balai Kota Kecil.",
        (ProjectId::CityHall, Lang::En) => "Advances your city to Town (unlocks Mining & Lv 30 cap; requires Market Hall).",
        (ProjectId::CityHall, Lang::Id) => "Naikkan kota ke Kota Kecil (buka Mining & cap Lv 30; butuh Pasar Desa).",
        (ProjectId::TradePort, Lang::En) => "Advances your city to Trade City (Lv 40 activity cap).",
        (ProjectId::TradePort, Lang::Id) => "Naikkan kota ke Kota Dagang (cap aktivitas Lv 40).",
        (ProjectId::MetropolisTower, Lang::En) => "Advances your city to Metropolis (Lv 50 activity cap).",
        (ProjectId::MetropolisTower, Lang::Id) => "Naikkan kota ke Metropolis (cap aktivitas Lv 50).",
    }
}

/// Translates either a project key or its Indonesian name into the requested language.
pub fn project_name_by_any(raw: &str, lang: Lang) -> &str {
    if let Some(pid) = ProjectId::parse(raw) {
        return project_name(pid, lang);
    }
    for &pid in &ProjectId::ALL {
        if crate::domain::city::def(pid).name == raw {
            return project_name(pid, lang);
        }
    }
    raw
}

pub fn activity_name(kind: ActivityKind, _lang: Lang) -> &'static str {
    kind.name()
}

pub fn recipe_name(id: &str, lang: Lang) -> &'static str {
    match (id, lang) {
        ("flour", Lang::En) => "Wheat Flour",
        ("flour", Lang::Id) => "Tepung Gandum",
        ("plank", Lang::En) => "Wood Plank",
        ("plank", Lang::Id) => "Papan Kayu",
        ("herb_extract", Lang::En) => "Herb Extract",
        ("herb_extract", Lang::Id) => "Ekstrak Herba",
        ("bread", Lang::En) => "Village Bread",
        ("bread", Lang::Id) => "Roti Kampung",
        ("wooden_crate", Lang::En) => "Wooden Crate",
        ("wooden_crate", Lang::Id) => "Peti Kayu",
        ("healing_tonic", Lang::En) => "Herbal Tonic",
        ("healing_tonic", Lang::Id) => "Tonik Herba",
        _ => item_name(id, lang),
    }
}

pub fn recipe_short_name(id: &str, lang: Lang) -> &'static str {
    match (id, lang) {
        ("flour", Lang::En) => "Flour",
        ("flour", Lang::Id) => "Tepung",
        ("plank", Lang::En) => "Plank",
        ("plank", Lang::Id) => "Papan",
        ("herb_extract", Lang::En) => "Extract",
        ("herb_extract", Lang::Id) => "Ekstrak",
        ("bread", Lang::En) => "Bread",
        ("bread", Lang::Id) => "Roti",
        ("wooden_crate", Lang::En) => "Crate",
        ("wooden_crate", Lang::Id) => "Peti",
        ("healing_tonic", Lang::En) => "Tonic",
        ("healing_tonic", Lang::Id) => "Tonik",
        _ => recipe_name(id, lang),
    }
}

pub fn recipe_tier_name(tier: RecipeTier, lang: Lang) -> &'static str {
    match (tier, lang) {
        (RecipeTier::Processed, Lang::En) => "Processed",
        (RecipeTier::Processed, Lang::Id) => "Olahan",
        (RecipeTier::Finished, Lang::En) => "Finished",
        (RecipeTier::Finished, Lang::Id) => "Produk Jadi",
    }
}

pub fn season_name(season: Season, lang: Lang) -> &'static str {
    match (season, lang) {
        (Season::Spring, Lang::En) => "Spring",
        (Season::Spring, Lang::Id) => "Musim Semi",
        (Season::Summer, Lang::En) => "Summer",
        (Season::Summer, Lang::Id) => "Musim Panas",
        (Season::Autumn, Lang::En) => "Autumn",
        (Season::Autumn, Lang::Id) => "Musim Gugur",
        (Season::Winter, Lang::En) => "Winter",
        (Season::Winter, Lang::Id) => "Musim Dingin",
    }
}

pub fn world_event_name(event: WorldEvent, lang: Lang) -> &'static str {
    match (event, lang) {
        (WorldEvent::HarvestFestival, Lang::En) => "Harvest Festival",
        (WorldEvent::HarvestFestival, Lang::Id) => "Festival Panen",
        (WorldEvent::MerchantCaravan, Lang::En) => "Merchant Caravan",
        (WorldEvent::MerchantCaravan, Lang::Id) => "Karavan Dagang",
        (WorldEvent::StormShortage, Lang::En) => "Storm Shortage",
        (WorldEvent::StormShortage, Lang::Id) => "Badai Laut & Hutan",
        (WorldEvent::MiningBoom, Lang::En) => "Mining Boom",
        (WorldEvent::MiningBoom, Lang::Id) => "Demam Tambang",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::crafting::RECIPES;

    #[test]
    fn lang_roundtrip_and_toggle() {
        assert_eq!(Lang::parse("en"), Lang::En);
        assert_eq!(Lang::parse("id"), Lang::Id);
        assert_eq!(Lang::parse("ID"), Lang::Id);
        assert_eq!(Lang::parse("other"), Lang::En);
        assert_eq!(Lang::En.toggle(), Lang::Id);
        assert_eq!(Lang::Id.toggle(), Lang::En);
        assert_eq!(Lang::En.as_str(), "en");
        assert_eq!(Lang::Id.as_str(), "id");
    }

    #[test]
    fn all_domain_entries_have_translations() {
        for item in ITEMS {
            assert!(!item_name(item.id, Lang::En).is_empty());
            assert!(!item_name(item.id, Lang::Id).is_empty());
            assert_eq!(item_name_by_any(item.name, Lang::En), item_name(item.id, Lang::En));
            assert_eq!(item_name_by_any(item.id, Lang::Id), item_name(item.id, Lang::Id));
        }

        for recipe in RECIPES {
            assert!(!recipe_name(recipe.id, Lang::En).is_empty());
            assert!(!recipe_name(recipe.id, Lang::Id).is_empty());
            assert!(!recipe_short_name(recipe.id, Lang::En).is_empty());
            assert!(!recipe_short_name(recipe.id, Lang::Id).is_empty());
        }

        for pid in ProjectId::ALL {
            assert!(!project_name(pid, Lang::En).is_empty());
            assert!(!project_name(pid, Lang::Id).is_empty());
            assert!(!project_desc(pid, Lang::En).is_empty());
            assert!(!project_desc(pid, Lang::Id).is_empty());
            let id_name = crate::domain::city::def(pid).name;
            assert_eq!(project_name_by_any(id_name, Lang::En), project_name(pid, Lang::En));
        }

        for stage in [
            CityStage::Kampung,
            CityStage::Desa,
            CityStage::KotaKecil,
            CityStage::KotaDagang,
            CityStage::Metropolis,
        ] {
            assert!(!stage_name(stage, Lang::En).is_empty());
            assert!(!stage_name(stage, Lang::Id).is_empty());
            assert!(!stage_era(stage, Lang::En).is_empty());
            assert!(!stage_era(stage, Lang::Id).is_empty());
        }

        for s in [Season::Spring, Season::Summer, Season::Autumn, Season::Winter] {
            assert!(!season_name(s, Lang::En).is_empty());
            assert!(!season_name(s, Lang::Id).is_empty());
        }

        for ev in WorldEvent::ALL {
            assert!(!world_event_name(ev, Lang::En).is_empty());
            assert!(!world_event_name(ev, Lang::Id).is_empty());
        }
    }
}
