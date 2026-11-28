use crate::domain::activities::ActivityKind;
use crate::domain::city::{CityStage, ProjectId};
use crate::domain::crafting::RecipeTier;
use crate::domain::resources::{find_item, ITEMS};
use crate::domain::world::{Season, WorldEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
}

impl Lang {
    pub fn parse(_s: &str) -> Self {
        Lang::En
    }

    pub fn as_str(&self) -> &'static str {
        "en"
    }

    pub fn label(&self) -> &'static str {
        "English"
    }
}

pub fn item_name(id: &str, _lang: Lang) -> &'static str {
    match id {
        "wheat" => "Wheat",
        "potato" => "Potato",
        "wood" => "Wood",
        "berries" => "Forest Berries",
        "wild_herb" => "Wild Herb",
        "flour" => "Wheat Flour",
        "plank" => "Wood Plank",
        "herb_extract" => "Herb Extract",
        "bread" => "Village Bread",
        "wooden_crate" => "Wooden Crate",
        "healing_tonic" => "Herbal Tonic",
        "river_fish" => "River Fish",
        "silver_carp" => "Silver Carp",
        "golden_eel" => "Golden Eel",
        "stone" => "Stone",
        "iron_ore" => "Iron Ore",
        "amethyst" => "Amethyst",
        "star_crystal" => "Star Crystal",
        _ => find_item(id).map(|i| i.name).unwrap_or("?"),
    }
}

/// Translates either an item ID or its catalog name into English.
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

pub fn stage_name(stage: CityStage, _lang: Lang) -> &'static str {
    stage.name()
}

pub fn stage_era(stage: CityStage, _lang: Lang) -> &'static str {
    stage.era()
}

pub fn project_name(id: ProjectId, _lang: Lang) -> &'static str {
    crate::domain::city::def(id).name
}

pub fn project_desc(id: ProjectId, _lang: Lang) -> &'static str {
    crate::domain::city::def(id).description
}

/// Translates either a project key or its catalog name into English.
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
    match id {
        "flour" => "Wheat Flour",
        "plank" => "Wood Plank",
        "herb_extract" => "Herb Extract",
        "bread" => "Village Bread",
        "wooden_crate" => "Wooden Crate",
        "healing_tonic" => "Herbal Tonic",
        _ => item_name(id, lang),
    }
}

pub fn recipe_short_name(id: &str, lang: Lang) -> &'static str {
    match id {
        "flour" => "Flour",
        "plank" => "Plank",
        "herb_extract" => "Extract",
        "bread" => "Bread",
        "wooden_crate" => "Crate",
        "healing_tonic" => "Tonic",
        _ => recipe_name(id, lang),
    }
}

pub fn recipe_tier_name(tier: RecipeTier, _lang: Lang) -> &'static str {
    tier.name()
}

pub fn season_name(season: Season, _lang: Lang) -> &'static str {
    season.name()
}

pub fn world_event_name(event: WorldEvent, _lang: Lang) -> &'static str {
    event.name()
}

pub fn season_bonus_hint(season: Season, _lang: Lang) -> &'static str {
    match season {
        Season::Spring => "+10% Potato, Berries, Herb",
        Season::Summer => "+12% Fish",
        Season::Autumn => "+15% Wheat, Flour, Bread",
        Season::Winter => "+15% Wood, Plank, Minerals",
    }
}

pub fn event_bonus_hint(event: WorldEvent, _lang: Lang) -> &'static str {
    match event {
        WorldEvent::HarvestFestival => "+20% Wheat, Potato, Bread",
        WorldEvent::MerchantCaravan => "+15% Bread, Crate, Tonic",
        WorldEvent::StormShortage => "+20% Wood, River Fish, Silver Carp",
        WorldEvent::MiningBoom => "+20% Mining Minerals",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::crafting::RECIPES;

    #[test]
    fn lang_always_resolves_to_english() {
        assert_eq!(Lang::parse("en"), Lang::En);
        assert_eq!(Lang::parse("id"), Lang::En);
        assert_eq!(Lang::parse("ID"), Lang::En);
        assert_eq!(Lang::parse("other"), Lang::En);
        assert_eq!(Lang::En.as_str(), "en");
        assert_eq!(Lang::En.label(), "English");
    }

    #[test]
    fn all_domain_entries_have_translations() {
        for item in ITEMS {
            assert!(!item_name(item.id, Lang::En).is_empty());
            assert_eq!(item_name_by_any(item.name, Lang::En), item_name(item.id, Lang::En));
            assert_eq!(item_name_by_any(item.id, Lang::En), item_name(item.id, Lang::En));
        }

        for recipe in RECIPES {
            assert!(!recipe_name(recipe.id, Lang::En).is_empty());
            assert!(!recipe_short_name(recipe.id, Lang::En).is_empty());
        }

        for pid in ProjectId::ALL {
            assert!(!project_name(pid, Lang::En).is_empty());
            assert!(!project_desc(pid, Lang::En).is_empty());
            let name = crate::domain::city::def(pid).name;
            assert_eq!(project_name_by_any(name, Lang::En), project_name(pid, Lang::En));
        }

        for stage in [
            CityStage::Kampung,
            CityStage::Desa,
            CityStage::KotaKecil,
            CityStage::KotaDagang,
            CityStage::Metropolis,
        ] {
            assert!(!stage_name(stage, Lang::En).is_empty());
            assert!(!stage_era(stage, Lang::En).is_empty());
        }

        for s in [Season::Spring, Season::Summer, Season::Autumn, Season::Winter] {
            assert!(!season_name(s, Lang::En).is_empty());
        }

        for ev in WorldEvent::ALL {
            assert!(!world_event_name(ev, Lang::En).is_empty());
        }
    }
}
