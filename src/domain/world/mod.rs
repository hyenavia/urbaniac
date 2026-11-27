use rust_decimal::Decimal;

pub const DAYS_PER_SEASON: i32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub fn as_str(&self) -> &'static str {
        match self {
            Season::Spring => "spring",
            Season::Summer => "summer",
            Season::Autumn => "autumn",
            Season::Winter => "winter",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "spring" => Some(Season::Spring),
            "summer" => Some(Season::Summer),
            "autumn" => Some(Season::Autumn),
            "winter" => Some(Season::Winter),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Season::Spring => "Musim Semi",
            Season::Summer => "Musim Panas",
            Season::Autumn => "Musim Gugur",
            Season::Winter => "Musim Dingin",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Season::Spring => Season::Summer,
            Season::Summer => Season::Autumn,
            Season::Autumn => Season::Winter,
            Season::Winter => Season::Spring,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldEvent {
    HarvestFestival,
    MerchantCaravan,
    StormShortage,
    MiningBoom,
}

impl WorldEvent {
    pub const ALL: [WorldEvent; 4] = [
        WorldEvent::HarvestFestival,
        WorldEvent::MerchantCaravan,
        WorldEvent::StormShortage,
        WorldEvent::MiningBoom,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            WorldEvent::HarvestFestival => "harvest_festival",
            WorldEvent::MerchantCaravan => "merchant_caravan",
            WorldEvent::StormShortage => "storm_shortage",
            WorldEvent::MiningBoom => "mining_boom",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|e| e.as_str() == s)
    }

    pub fn name(&self) -> &'static str {
        match self {
            WorldEvent::HarvestFestival => "Festival Panen",
            WorldEvent::MerchantCaravan => "Karavan Dagang",
            WorldEvent::StormShortage => "Badai Laut & Hutan",
            WorldEvent::MiningBoom => "Demam Tambang",
        }
    }
}

pub fn season_modifier(season: Season, item_id: &str) -> Decimal {
    match (season, item_id) {
        (Season::Spring, "potato" | "berries" | "wild_herb") => Decimal::new(11_000, 4),
        (Season::Summer, "river_fish" | "silver_carp" | "golden_eel") => Decimal::new(11_200, 4),
        (Season::Autumn, "wheat" | "flour" | "bread") => Decimal::new(11_500, 4),
        (Season::Winter, "wood" | "plank" | "stone" | "iron_ore" | "amethyst" | "star_crystal") => {
            Decimal::new(11_500, 4)
        }
        _ => Decimal::ONE,
    }
}

pub fn event_modifier(event: Option<WorldEvent>, item_id: &str) -> Decimal {
    match (event, item_id) {
        (Some(WorldEvent::HarvestFestival), "wheat" | "potato" | "bread") => Decimal::new(12_000, 4),
        (Some(WorldEvent::MerchantCaravan), "bread" | "wooden_crate" | "healing_tonic") => {
            Decimal::new(11_500, 4)
        }
        (Some(WorldEvent::StormShortage), "wood" | "river_fish" | "silver_carp") => {
            Decimal::new(12_000, 4)
        }
        (Some(WorldEvent::MiningBoom), "stone" | "iron_ore" | "amethyst" | "star_crystal") => {
            Decimal::new(12_000, 4)
        }
        _ => Decimal::ONE,
    }
}

pub fn world_price_multiplier(
    season: Season,
    event: Option<WorldEvent>,
    trend_item_id: Option<&str>,
    item_id: &str,
) -> Decimal {
    let sm = season_modifier(season, item_id);
    let em = event_modifier(event, item_id);
    let tm = if trend_item_id == Some(item_id) {
        Decimal::new(11_500, 4)
    } else {
        Decimal::ONE
    };
    (sm * em * tm).round_dp(4)
}

/// Memajukan hari kalender global. Mengembalikan `(new_day, new_season, season_changed)`.
pub fn advance_calendar_day(day: i32, season: Season) -> (i32, Season, bool) {
    let next_day = day.max(1) + 1;
    if (next_day - 1) % DAYS_PER_SEASON == 0 {
        (next_day, season.next(), true)
    } else {
        (next_day, season, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_rolls_season_after_seven_days() {
        let (d, s, changed) = advance_calendar_day(7, Season::Spring);
        assert_eq!(d, 8);
        assert_eq!(s, Season::Summer);
        assert!(changed);

        let (d2, s2, changed2) = advance_calendar_day(2, Season::Spring);
        assert_eq!((d2, s2, changed2), (3, Season::Spring, false));
    }

    #[test]
    fn world_price_multiplier_combines_season_event_and_trend() {
        // Spring wheat default = 1.0000
        assert_eq!(
            world_price_multiplier(Season::Spring, None, None, "wheat"),
            Decimal::new(10_000, 4)
        );
        // Autumn (1.15) * HarvestFestival (1.20) * Trend (1.15) = 1.5870
        assert_eq!(
            world_price_multiplier(
                Season::Autumn,
                Some(WorldEvent::HarvestFestival),
                Some("wheat"),
                "wheat"
            ),
            Decimal::new(15_870, 4)
        );
    }
}
