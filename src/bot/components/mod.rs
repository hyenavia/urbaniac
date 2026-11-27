use serenity::all::*;

pub const PREFIX: &str = "ub";

fn btn(action: &str, owner: u64, rest: &str, label: &str, style: ButtonStyle) -> CreateButton {
    let id = if rest.is_empty() {
        format!("{PREFIX}:{action}:{owner}")
    } else {
        format!("{PREFIX}:{action}:{owner}:{rest}")
    };
    CreateButton::new(id).label(label).style(style)
}

pub fn main_menu(owner: u64) -> Vec<CreateActionRow> {
    vec![
        CreateActionRow::Buttons(vec![
            btn("profile", owner, "", "Profil", ButtonStyle::Primary),
            btn("inventory", owner, "", "Inventory", ButtonStyle::Secondary),
            btn("activities", owner, "", "Aktivitas", ButtonStyle::Primary),
            btn("city", owner, "", "Kota", ButtonStyle::Primary),
        ]),
        CreateActionRow::Buttons(vec![
            btn("market", owner, "", "Pasar & Makelar", ButtonStyle::Success),
            btn("sell", owner, "", "Jual NPC", ButtonStyle::Success),
            btn("claim", owner, "", "Klaim Pending", ButtonStyle::Secondary),
        ]),
    ]
}

pub fn city_menu(owner: u64) -> Vec<CreateActionRow> {
    vec![
        CreateActionRow::Buttons(vec![
            btn("cityproj", owner, "well", "Bangun Sumur", ButtonStyle::Success),
            btn("cityproj", owner, "granary", "Bangun Lumbung", ButtonStyle::Success),
            btn("cityproj", owner, "town_hall", "Bangun Balai Kampung", ButtonStyle::Success),
        ]),
        CreateActionRow::Buttons(vec![
            btn("citydep", owner, "50", "Setor 50 Bac", ButtonStyle::Primary),
            btn("city", owner, "", "Refresh", ButtonStyle::Secondary),
            btn("profile", owner, "", "Menu Utama", ButtonStyle::Secondary),
        ]),
    ]
}

pub fn activity_menu(owner: u64) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        btn("startact", owner, "farming", "Mulai Farming", ButtonStyle::Success),
        btn("startact", owner, "gathering", "Mulai Gathering", ButtonStyle::Success),
        btn("collect", owner, "", "Ambil Reward", ButtonStyle::Primary),
        btn("activities", owner, "", "Refresh", ButtonStyle::Secondary),
        btn("profile", owner, "", "Menu Utama", ButtonStyle::Secondary),
    ])]
}

/// `items`: (nama, item_id, jumlah)
pub fn sell_select(owner: u64, items: &[(String, String, i64)]) -> Vec<CreateActionRow> {
    let options: Vec<CreateSelectMenuOption> = items
        .iter()
        .take(25)
        .map(|(name, id, qty)| CreateSelectMenuOption::new(format!("{name} (x{qty})"), id.clone()))
        .collect();
    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                format!("{PREFIX}:sellpick:{owner}"),
                CreateSelectMenuKind::String { options },
            )
            .placeholder("Pilih item untuk dijual"),
        ),
        CreateActionRow::Buttons(vec![btn("profile", owner, "", "Menu Utama", ButtonStyle::Secondary)]),
    ]
}

pub fn sell_qty(owner: u64, item_id: &str) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        btn("sellqty", owner, &format!("{item_id}:1"), "Jual 1", ButtonStyle::Success),
        btn("sellqty", owner, &format!("{item_id}:10"), "Jual 10", ButtonStyle::Success),
        btn("sellqty", owner, &format!("{item_id}:all"), "Jual Semua", ButtonStyle::Danger),
        btn("sell", owner, "", "Kembali", ButtonStyle::Secondary),
    ])]
}

pub fn market_menu(owner: u64, has_listings: bool) -> Vec<CreateActionRow> {
    let mut buttons = vec![btn("mktnew", owner, "", "Titip ke Makelar", ButtonStyle::Success)];
    if has_listings {
        buttons.push(btn("mktcancel", owner, "", "Batalkan Listing", ButtonStyle::Danger));
    }
    buttons.push(btn("market", owner, "", "Refresh", ButtonStyle::Secondary));
    buttons.push(btn("profile", owner, "", "Menu Utama", ButtonStyle::Secondary));
    vec![CreateActionRow::Buttons(buttons)]
}

pub fn broker_item_select(owner: u64, items: &[(String, String, i64)]) -> Vec<CreateActionRow> {
    let options: Vec<CreateSelectMenuOption> = items
        .iter()
        .take(25)
        .map(|(name, id, qty)| CreateSelectMenuOption::new(format!("{name} (x{qty})"), id.clone()))
        .collect();
    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                format!("{PREFIX}:mktpick:{owner}"),
                CreateSelectMenuKind::String { options },
            )
            .placeholder("Pilih item untuk dititipkan"),
        ),
        CreateActionRow::Buttons(vec![btn("market", owner, "", "Kembali", ButtonStyle::Secondary)]),
    ]
}

pub fn broker_qty_select(owner: u64, item_id: &str, owned: i64) -> Vec<CreateActionRow> {
    let mut buttons = Vec::new();
    for q in [1, 5, 10, 25] {
        if owned >= q {
            buttons.push(btn(
                "mktqty",
                owner,
                &format!("{item_id}:{q}"),
                &format!("Titip {q}"),
                ButtonStyle::Primary,
            ));
        }
    }
    if owned > 0 && !matches!(owned, 1 | 5 | 10 | 25) {
        buttons.push(btn(
            "mktqty",
            owner,
            &format!("{item_id}:{owned}"),
            &format!("Semua ({owned})"),
            ButtonStyle::Primary,
        ));
    }
    vec![
        CreateActionRow::Buttons(buttons),
        CreateActionRow::Buttons(vec![btn("market", owner, "", "Kembali", ButtonStyle::Secondary)]),
    ]
}

pub fn broker_terms_select(owner: u64, item_id: &str, qty: i64) -> Vec<CreateActionRow> {
    vec![
        CreateActionRow::Buttons(vec![
            btn("mktpost", owner, &format!("{item_id}:{qty}:110:1h"), "+10% (1 Jam)", ButtonStyle::Success),
            btn("mktpost", owner, &format!("{item_id}:{qty}:125:6h"), "+25% (6 Jam)", ButtonStyle::Success),
            btn("mktpost", owner, &format!("{item_id}:{qty}:140:24h"), "+40% (24 Jam)", ButtonStyle::Success),
        ]),
        CreateActionRow::Buttons(vec![btn("market", owner, "", "Kembali", ButtonStyle::Secondary)]),
    ]
}

pub fn broker_cancel_select(owner: u64, listings: &[(i64, String)]) -> Vec<CreateActionRow> {
    let options: Vec<CreateSelectMenuOption> = listings
        .iter()
        .take(25)
        .map(|(id, label)| CreateSelectMenuOption::new(label.clone(), id.to_string()))
        .collect();
    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                format!("{PREFIX}:mktcancelpick:{owner}"),
                CreateSelectMenuKind::String { options },
            )
            .placeholder("Pilih listing untuk dibatalkan"),
        ),
        CreateActionRow::Buttons(vec![btn("market", owner, "", "Kembali", ButtonStyle::Secondary)]),
    ]
}

/// Format custom_id: `ub:<aksi>:<owner_discord_id>[:<data>]`. Mengembalikan (aksi, owner, data).
pub fn parse_custom_id(id: &str) -> Option<(&str, u64, &str)> {
    let mut parts = id.splitn(4, ':');
    if parts.next()? != PREFIX {
        return None;
    }
    let action = parts.next()?;
    let owner = parts.next()?.parse::<u64>().ok()?;
    let rest = parts.next().unwrap_or("");
    Some((action, owner, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_id() {
        assert_eq!(parse_custom_id("ub:profile:123"), Some(("profile", 123, "")));
        assert_eq!(parse_custom_id("ub:sellqty:5:wild_herb:10"), Some(("sellqty", 5, "wild_herb:10")));
    }

    #[test]
    fn rejects_invalid_ids() {
        assert_eq!(parse_custom_id("x:profile:123"), None);
        assert_eq!(parse_custom_id("ub:profile"), None);
        assert_eq!(parse_custom_id("ub:profile:abc"), None);
    }
}
