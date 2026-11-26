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
}

/// Slot dasar inventory (Lumbung akan menambah di tahap City). Nilai awal sementara.
pub const BASE_SLOT_CAPACITY: i64 = 12;

pub const ITEMS: &[ItemDef] = &[
    ItemDef { id: "wheat", name: "Gandum", rarity: Rarity::Common, stack_limit: 100 },
    ItemDef { id: "potato", name: "Kentang", rarity: Rarity::Common, stack_limit: 100 },
    ItemDef { id: "wood", name: "Kayu", rarity: Rarity::Common, stack_limit: 100 },
    ItemDef { id: "berries", name: "Buah Hutan", rarity: Rarity::Common, stack_limit: 100 },
    ItemDef { id: "wild_herb", name: "Herba Liar", rarity: Rarity::Uncommon, stack_limit: 50 },
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
