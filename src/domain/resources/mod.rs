use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl fmt::Display for Rarity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Rarity::Common => "Common",
            Rarity::Uncommon => "Uncommon",
            Rarity::Rare => "Rare",
            Rarity::Epic => "Epic",
            Rarity::Legendary => "Legendary",
        };
        f.write_str(s)
    }
}

#[derive(Debug)]
pub struct ItemDef {
    pub id: &'static str,
    pub name: &'static str,
    pub rarity: Rarity,
    pub stack_limit: i64,
    /// Harga beli NPC per unit, dalam sen Bac (300 = 3.00 Bac). Nilai awal sementara.
    pub npc_price_cents: i64,
}

/// Slot dasar inventory (Lumbung akan menambah di tahap City). Nilai awal sementara.
pub const BASE_SLOT_CAPACITY: i64 = 12;

pub const ITEMS: &[ItemDef] = &[
    // Raw
    ItemDef { id: "wheat", name: "Gandum", rarity: Rarity::Common, stack_limit: 100, npc_price_cents: 300 },
    ItemDef { id: "potato", name: "Kentang", rarity: Rarity::Common, stack_limit: 100, npc_price_cents: 400 },
    ItemDef { id: "wood", name: "Kayu", rarity: Rarity::Common, stack_limit: 100, npc_price_cents: 300 },
    ItemDef { id: "berries", name: "Buah Hutan", rarity: Rarity::Common, stack_limit: 100, npc_price_cents: 350 },
    ItemDef { id: "wild_herb", name: "Herba Liar", rarity: Rarity::Uncommon, stack_limit: 50, npc_price_cents: 1200 },
    // Processed
    ItemDef { id: "flour", name: "Tepung Gandum", rarity: Rarity::Common, stack_limit: 100, npc_price_cents: 800 },
    ItemDef { id: "plank", name: "Papan Kayu", rarity: Rarity::Common, stack_limit: 100, npc_price_cents: 800 },
    ItemDef { id: "herb_extract", name: "Ekstrak Herba", rarity: Rarity::Uncommon, stack_limit: 50, npc_price_cents: 3200 },
    // Finished
    ItemDef { id: "bread", name: "Roti Kampung", rarity: Rarity::Common, stack_limit: 50, npc_price_cents: 2000 },
    ItemDef { id: "wooden_crate", name: "Peti Kayu", rarity: Rarity::Uncommon, stack_limit: 50, npc_price_cents: 2200 },
    ItemDef { id: "healing_tonic", name: "Tonik Herba", rarity: Rarity::Rare, stack_limit: 30, npc_price_cents: 5500 },
    // Fishing (Desa)
    ItemDef { id: "river_fish", name: "Ikan Sungai", rarity: Rarity::Common, stack_limit: 100, npc_price_cents: 500 },
    ItemDef { id: "silver_carp", name: "Gurame Perak", rarity: Rarity::Uncommon, stack_limit: 50, npc_price_cents: 1600 },
    ItemDef { id: "golden_eel", name: "Belut Emas", rarity: Rarity::Rare, stack_limit: 30, npc_price_cents: 4500 },
    // Mining (Kota Kecil)
    ItemDef { id: "stone", name: "Batu", rarity: Rarity::Common, stack_limit: 100, npc_price_cents: 450 },
    ItemDef { id: "iron_ore", name: "Bijih Besi", rarity: Rarity::Uncommon, stack_limit: 50, npc_price_cents: 1800 },
    ItemDef { id: "amethyst", name: "Kecubung", rarity: Rarity::Rare, stack_limit: 30, npc_price_cents: 6000 },
    ItemDef { id: "star_crystal", name: "Kristal Bintang", rarity: Rarity::Epic, stack_limit: 20, npc_price_cents: 15000 },
];

pub fn find_item(id: &str) -> Option<&'static ItemDef> {
    ITEMS.iter().find(|i| i.id == id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub stored: i64,
    pub pending: i64,
}

/// Aturan: satu jenis item = satu slot, ditumpuk sampai `stack_limit`.
/// Item yang tidak muat (stack penuh atau tidak ada slot baru) menjadi pending.
pub fn plan_add(
    existing_qty: Option<i64>,
    used_slots: i64,
    slot_capacity: i64,
    stack_limit: i64,
    qty: i64,
) -> Placement {
    if qty <= 0 {
        return Placement { stored: 0, pending: 0 };
    }
    let room = match existing_qty {
        Some(q) => (stack_limit - q).max(0),
        None if used_slots < slot_capacity => stack_limit,
        None => 0,
    };
    let stored = qty.min(room);
    Placement { stored, pending: qty - stored }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_item_fits_in_free_slot() {
        assert_eq!(plan_add(None, 3, 12, 100, 40), Placement { stored: 40, pending: 0 });
    }

    #[test]
    fn stack_limit_overflows_to_pending() {
        assert_eq!(plan_add(Some(90), 3, 12, 100, 30), Placement { stored: 10, pending: 20 });
    }

    #[test]
    fn no_free_slot_means_all_pending() {
        assert_eq!(plan_add(None, 12, 12, 100, 5), Placement { stored: 0, pending: 5 });
    }

    #[test]
    fn existing_stack_does_not_need_free_slot() {
        assert_eq!(plan_add(Some(10), 12, 12, 100, 5), Placement { stored: 5, pending: 0 });
    }

    #[test]
    fn catalog_ids_unique_and_found() {
        for (i, a) in ITEMS.iter().enumerate() {
            assert!(ITEMS.iter().skip(i + 1).all(|b| b.id != a.id));
            assert!(find_item(a.id).is_some());
        }
        assert!(find_item("nope").is_none());
    }
}
