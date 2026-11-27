use crate::domain::activities::ActivityKind;
use crate::domain::city::{def as project_def, CityStage, ProjectId};
use crate::domain::economy::format_bac;
use crate::domain::i18n::{activity_name, project_name, recipe_short_name, Lang};
use rust_decimal::Decimal;
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

pub fn main_menu(owner: u64, lang: Lang) -> Vec<CreateActionRow> {
    let (l_prof, l_inv, l_act, l_city, l_mkt, l_craft, l_crew, l_guide) = match lang {
        Lang::En => (
            "Profile",
            "Inventory",
            "Activities",
            "City",
            "Market & Broker",
            "Crafting & Orders",
            "Crew",
            "Guide",
        ),
        Lang::Id => (
            "Profil",
            "Inventory",
            "Aktivitas",
            "Kota",
            "Pasar & Makelar",
            "Crafting & Pesanan",
            "Kru",
            "Panduan",
        ),
    };
    vec![
        CreateActionRow::Buttons(vec![
            btn("profile", owner, "", l_prof, ButtonStyle::Primary),
            btn("inventory", owner, "", l_inv, ButtonStyle::Secondary),
            btn("activities", owner, "", l_act, ButtonStyle::Primary),
            btn("city", owner, "", l_city, ButtonStyle::Primary),
        ]),
        CreateActionRow::Buttons(vec![
            btn("market", owner, "", l_mkt, ButtonStyle::Success),
            btn("craft", owner, "", l_craft, ButtonStyle::Success),
            btn("crew", owner, "", l_crew, ButtonStyle::Primary),
            btn("guide", owner, "", l_guide, ButtonStyle::Secondary),
        ]),
    ]
}

pub fn profile_menu(owner: u64, lang: Lang) -> Vec<CreateActionRow> {
    let l_lang = match lang {
        Lang::En => "🌐 Language: EN (Switch to ID)",
        Lang::Id => "🌐 Bahasa: ID (Ganti ke EN)",
    };
    let mut rows = main_menu(owner, lang);
    rows.push(CreateActionRow::Buttons(vec![btn(
        "lang",
        owner,
        "",
        l_lang,
        ButtonStyle::Secondary,
    )]));
    rows
}

pub fn inventory_menu(
    owner: u64,
    has_items: bool,
    has_pending: bool,
    lang: Lang,
) -> Vec<CreateActionRow> {
    let (sell_lbl, claim_lbl, menu_lbl) = match lang {
        Lang::En => ("Sell to NPC", "Claim Pending", "Main Menu"),
        Lang::Id => ("Jual ke NPC", "Klaim Pending", "Menu Utama"),
    };
    let mut buttons = vec![
        btn("sell", owner, "", sell_lbl, ButtonStyle::Success).disabled(!has_items),
    ];
    if has_pending {
        buttons.push(btn("claim", owner, "", claim_lbl, ButtonStyle::Primary));
    }
    buttons.push(btn("inventory", owner, "", "Refresh", ButtonStyle::Secondary));
    buttons.push(btn("profile", owner, "", menu_lbl, ButtonStyle::Secondary));
    vec![CreateActionRow::Buttons(buttons)]
}

pub fn city_menu(
    owner: u64,
    stage: CityStage,
    built_levels: &[(ProjectId, i32)],
    lang: Lang,
) -> Vec<CreateActionRow> {
    let proj_buttons: Vec<CreateButton> = ProjectId::ALL
        .iter()
        .filter_map(|&pid| {
            let p = project_def(pid);
            let cur = built_levels.iter().find(|(id, _)| *id == pid).map(|(_, l)| *l).unwrap_or(0);
            if stage < p.min_stage || cur >= p.max_level {
                return None;
            }
            let p_name = project_name(pid, lang);
            let label = if pid == ProjectId::Granary && cur > 0 {
                format!("Upgrade {p_name} Lv {}", cur + 1)
            } else {
                match lang {
                    Lang::En => format!("Build {p_name}"),
                    Lang::Id => format!("Bangun {p_name}"),
                }
            };
            Some(btn("cityproj", owner, pid.as_str(), &label, ButtonStyle::Success))
        })
        .take(5)
        .collect();

    let (dep_lbl, wd_lbl, t5, t10, t15, menu_lbl) = match lang {
        Lang::En => (
            "Deposit",
            "Withdraw",
            "Tax 5%",
            "Tax 10%",
            "Tax 15%",
            "Main Menu",
        ),
        Lang::Id => (
            "Setor Custom",
            "Tarik Treasury",
            "Pajak 5%",
            "Pajak 10%",
            "Pajak 15%",
            "Menu Utama",
        ),
    };

    let mut rows = Vec::new();
    if !proj_buttons.is_empty() {
        rows.push(CreateActionRow::Buttons(proj_buttons));
    }
    rows.push(CreateActionRow::Buttons(vec![
        btn("citydepcustom", owner, "", dep_lbl, ButtonStyle::Primary),
        btn("citywdcustom", owner, "", wd_lbl, ButtonStyle::Primary),
        btn("citytax", owner, "5", t5, ButtonStyle::Secondary),
        btn("citytax", owner, "10", t10, ButtonStyle::Secondary),
        btn("citytax", owner, "15", t15, ButtonStyle::Secondary),
    ]));
    rows.push(CreateActionRow::Buttons(vec![
        btn("city", owner, "", "Refresh", ButtonStyle::Secondary),
        btn("profile", owner, "", menu_lbl, ButtonStyle::Secondary),
    ]));
    rows
}

pub fn city_deposit_modal(owner: u64, wallet_balance: Decimal, lang: Lang) -> CreateModal {
    let (label, ph, title) = match lang {
        Lang::En => (
            format!("Amount (Wallet: {})", format_bac(wallet_balance)),
            "e.g. 100 or max",
            "Deposit to City Treasury",
        ),
        Lang::Id => (
            format!("Jumlah (Saldo: {})", format_bac(wallet_balance)),
            "Contoh: 100 atau max",
            "Setor ke City Treasury",
        ),
    };
    let label: String = label.chars().take(45).collect();
    let input = CreateInputText::new(InputTextStyle::Short, label, "amount")
        .placeholder(ph)
        .required(true)
        .min_length(1)
        .max_length(20);
    CreateModal::new(format!("{PREFIX}:citydepmodal:{owner}"), title)
        .components(vec![CreateActionRow::InputText(input)])
}

pub fn city_withdraw_modal(owner: u64, treasury_balance: Decimal, lang: Lang) -> CreateModal {
    let (label, ph, title) = match lang {
        Lang::En => (
            format!("Amount (Treasury: {})", format_bac(treasury_balance)),
            "e.g. 100 or max",
            "Withdraw from City Treasury",
        ),
        Lang::Id => (
            format!("Jumlah (Treasury: {})", format_bac(treasury_balance)),
            "Contoh: 100 atau max",
            "Tarik dari City Treasury",
        ),
    };
    let label: String = label.chars().take(45).collect();
    let input = CreateInputText::new(InputTextStyle::Short, label, "amount")
        .placeholder(ph)
        .required(true)
        .min_length(1)
        .max_length(20);
    CreateModal::new(format!("{PREFIX}:citywdmodal:{owner}"), title)
        .components(vec![CreateActionRow::InputText(input)])
}

pub fn activity_menu(owner: u64, stage: CityStage, lang: Lang) -> Vec<CreateActionRow> {
    let (verb, collect, menu) = match lang {
        Lang::En => ("Start", "Collect Rewards", "Main Menu"),
        Lang::Id => ("Mulai", "Ambil Reward", "Menu Utama"),
    };
    let act_buttons: Vec<CreateButton> = ActivityKind::ALL
        .iter()
        .map(|&kind| {
            let locked = stage < kind.min_stage();
            let a_name = activity_name(kind, lang);
            let label = if locked {
                format!("🔒 {verb} {a_name}")
            } else {
                format!("{verb} {a_name}")
            };
            btn("startact", owner, kind.as_str(), &label, ButtonStyle::Success).disabled(locked)
        })
        .collect();
    vec![
        CreateActionRow::Buttons(act_buttons),
        CreateActionRow::Buttons(vec![
            btn("collect", owner, "", collect, ButtonStyle::Primary),
            btn("activities", owner, "", "Refresh", ButtonStyle::Secondary),
            btn("profile", owner, "", menu, ButtonStyle::Secondary),
        ]),
    ]
}

/// `items`: (nama, item_id, jumlah)
pub fn sell_select(
    owner: u64,
    items: &[(String, String, i64)],
    lang: Lang,
) -> Vec<CreateActionRow> {
    let options: Vec<CreateSelectMenuOption> = items
        .iter()
        .take(25)
        .map(|(name, id, qty)| CreateSelectMenuOption::new(format!("{name} (x{qty})"), id.clone()))
        .collect();
    let (ph, menu) = match lang {
        Lang::En => ("Select an item to sell", "Main Menu"),
        Lang::Id => ("Pilih item untuk dijual", "Menu Utama"),
    };
    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                format!("{PREFIX}:sellpick:{owner}"),
                CreateSelectMenuKind::String { options },
            )
            .placeholder(ph),
        ),
        CreateActionRow::Buttons(vec![btn("profile", owner, "", menu, ButtonStyle::Secondary)]),
    ]
}

pub fn sell_qty(owner: u64, item_id: &str, lang: Lang) -> Vec<CreateActionRow> {
    let (s1, s10, s100, sall, scust, back) = match lang {
        Lang::En => ("Sell 1", "Sell 10", "Sell 100", "Sell Max", "Sell Custom", "Back"),
        Lang::Id => ("Jual 1", "Jual 10", "Jual 100", "Jual Max", "Jual Custom", "Kembali"),
    };
    vec![
        CreateActionRow::Buttons(vec![
            btn("sellqty", owner, &format!("{item_id}:1"), s1, ButtonStyle::Success),
            btn("sellqty", owner, &format!("{item_id}:10"), s10, ButtonStyle::Success),
            btn("sellqty", owner, &format!("{item_id}:100"), s100, ButtonStyle::Success),
            btn("sellqty", owner, &format!("{item_id}:all"), sall, ButtonStyle::Danger),
            btn("sellcustom", owner, item_id, scust, ButtonStyle::Primary),
        ]),
        CreateActionRow::Buttons(vec![btn("sell", owner, "", back, ButtonStyle::Secondary)]),
    ]
}

pub fn sell_custom_modal(
    owner: u64,
    item_id: &str,
    item_name: &str,
    owned: i64,
    lang: Lang,
) -> CreateModal {
    let (label, ph, title) = match lang {
        Lang::En => (
            format!("Amount of {item_name} (own: {owned})"),
            "e.g. 5",
            format!("Sell Custom — {item_name}"),
        ),
        Lang::Id => (
            format!("Jumlah {item_name} (punya {owned})"),
            "Contoh: 5",
            format!("Jual Custom — {item_name}"),
        ),
    };
    let input = CreateInputText::new(InputTextStyle::Short, label, "qty")
        .placeholder(ph)
        .required(true)
        .min_length(1)
        .max_length(10);
    CreateModal::new(format!("{PREFIX}:sellmodal:{owner}:{item_id}"), title)
        .components(vec![CreateActionRow::InputText(input)])
}

pub fn market_menu(owner: u64, has_listings: bool, lang: Lang) -> Vec<CreateActionRow> {
    let (sell_lbl, new_lbl, cancel_lbl, menu_lbl) = match lang {
        Lang::En => ("Sell to NPC", "List with Broker", "Cancel Listing", "Main Menu"),
        Lang::Id => ("Jual ke NPC", "Titip ke Makelar", "Batalkan Listing", "Menu Utama"),
    };
    let mut buttons = vec![
        btn("sell", owner, "", sell_lbl, ButtonStyle::Success),
        btn("mktnew", owner, "", new_lbl, ButtonStyle::Primary),
    ];
    if has_listings {
        buttons.push(btn("mktcancel", owner, "", cancel_lbl, ButtonStyle::Danger));
    }
    buttons.push(btn("market", owner, "", "Refresh", ButtonStyle::Secondary));
    buttons.push(btn("profile", owner, "", menu_lbl, ButtonStyle::Secondary));
    vec![CreateActionRow::Buttons(buttons)]
}

pub fn broker_item_select(
    owner: u64,
    items: &[(String, String, i64)],
    lang: Lang,
) -> Vec<CreateActionRow> {
    let options: Vec<CreateSelectMenuOption> = items
        .iter()
        .take(25)
        .map(|(name, id, qty)| CreateSelectMenuOption::new(format!("{name} (x{qty})"), id.clone()))
        .collect();
    let (ph, back) = match lang {
        Lang::En => ("Select an item to consign", "Back"),
        Lang::Id => ("Pilih item untuk dititipkan", "Kembali"),
    };
    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                format!("{PREFIX}:mktpick:{owner}"),
                CreateSelectMenuKind::String { options },
            )
            .placeholder(ph),
        ),
        CreateActionRow::Buttons(vec![btn("market", owner, "", back, ButtonStyle::Secondary)]),
    ]
}

pub fn broker_qty_select(
    owner: u64,
    item_id: &str,
    owned: i64,
    lang: Lang,
) -> Vec<CreateActionRow> {
    let (verb, all_word, back) = match lang {
        Lang::En => ("List", "All", "Back"),
        Lang::Id => ("Titip", "Semua", "Kembali"),
    };
    let mut buttons = Vec::new();
    for q in [1, 5, 10, 25] {
        if owned >= q {
            buttons.push(btn(
                "mktqty",
                owner,
                &format!("{item_id}:{q}"),
                &format!("{verb} {q}"),
                ButtonStyle::Primary,
            ));
        }
    }
    if owned > 0 && !matches!(owned, 1 | 5 | 10 | 25) {
        buttons.push(btn(
            "mktqty",
            owner,
            &format!("{item_id}:{owned}"),
            &format!("{all_word} ({owned})"),
            ButtonStyle::Primary,
        ));
    }
    let mut rows = Vec::new();
    if !buttons.is_empty() {
        rows.push(CreateActionRow::Buttons(buttons));
    }
    rows.push(CreateActionRow::Buttons(vec![btn(
        "market",
        owner,
        "",
        back,
        ButtonStyle::Secondary,
    )]));
    rows
}

pub fn broker_terms_select(
    owner: u64,
    item_id: &str,
    qty: i64,
    lang: Lang,
) -> Vec<CreateActionRow> {
    let (l1, l6, l24, back) = match lang {
        Lang::En => ("+10% (2 Hrs)", "+20% (6 Hrs)", "+35% (24 Hrs)", "Back"),
        Lang::Id => ("+10% (2 Jam)", "+20% (6 Jam)", "+35% (24 Jam)", "Kembali"),
    };
    vec![
        CreateActionRow::Buttons(vec![
            btn("mktpost", owner, &format!("{item_id}:{qty}:110:2h"), l1, ButtonStyle::Success),
            btn("mktpost", owner, &format!("{item_id}:{qty}:120:6h"), l6, ButtonStyle::Success),
            btn("mktpost", owner, &format!("{item_id}:{qty}:135:24h"), l24, ButtonStyle::Success),
        ]),
        CreateActionRow::Buttons(vec![btn("market", owner, "", back, ButtonStyle::Secondary)]),
    ]
}

pub fn broker_cancel_select(
    owner: u64,
    listings: &[(i64, String)],
    lang: Lang,
) -> Vec<CreateActionRow> {
    let options: Vec<CreateSelectMenuOption> = listings
        .iter()
        .take(25)
        .map(|(id, label)| CreateSelectMenuOption::new(label.clone(), id.to_string()))
        .collect();
    let (ph, back) = match lang {
        Lang::En => ("Select a listing to cancel", "Back"),
        Lang::Id => ("Pilih listing untuk dibatalkan", "Kembali"),
    };
    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                format!("{PREFIX}:mktcancelpick:{owner}"),
                CreateSelectMenuKind::String { options },
            )
            .placeholder(ph),
        ),
        CreateActionRow::Buttons(vec![btn("market", owner, "", back, ButtonStyle::Secondary)]),
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

pub fn craft_menu(owner: u64, auto_sell: bool, lang: Lang) -> Vec<CreateActionRow> {
    let auto_label = if auto_sell {
        "Auto-Sell Craft: ON"
    } else {
        "Auto-Sell Craft: OFF"
    };
    let (fulfill_lbl, refresh_ord_lbl, menu_lbl) = match lang {
        Lang::En => ("Fulfill Order", "Reroll Orders", "Main Menu"),
        Lang::Id => ("Penuhi Pesanan", "Ganti Pesanan", "Menu Utama"),
    };
    vec![
        CreateActionRow::Buttons(vec![
            btn("craftpick", owner, "flour", &format!("Craft {}", recipe_short_name("flour", lang)), ButtonStyle::Success),
            btn("craftpick", owner, "plank", &format!("Craft {}", recipe_short_name("plank", lang)), ButtonStyle::Success),
            btn("craftpick", owner, "herb_extract", &format!("Craft {}", recipe_short_name("herb_extract", lang)), ButtonStyle::Success),
        ]),
        CreateActionRow::Buttons(vec![
            btn("craftpick", owner, "bread", &format!("Craft {}", recipe_short_name("bread", lang)), ButtonStyle::Primary),
            btn("craftpick", owner, "wooden_crate", &format!("Craft {}", recipe_short_name("wooden_crate", lang)), ButtonStyle::Primary),
            btn("craftpick", owner, "healing_tonic", &format!("Craft {}", recipe_short_name("healing_tonic", lang)), ButtonStyle::Primary),
        ]),
        CreateActionRow::Buttons(vec![
            btn("orderpick", owner, "", fulfill_lbl, ButtonStyle::Success),
            btn("orderrefresh", owner, "", refresh_ord_lbl, ButtonStyle::Primary),
            btn("craftauto", owner, "", auto_label, ButtonStyle::Secondary),
            btn("craft", owner, "", "Refresh", ButtonStyle::Secondary),
            btn("profile", owner, "", menu_lbl, ButtonStyle::Secondary),
        ]),
    ]
}

pub fn craft_qty(owner: u64, recipe_id: &str, max_batch: i64, lang: Lang) -> Vec<CreateActionRow> {
    let (c1, c5, c10, cmax, ccust, back) = match lang {
        Lang::En => (
            "Craft 1",
            "Craft 5",
            "Craft 10",
            format!("Craft Max ({max_batch})"),
            "Craft Custom",
            "Back",
        ),
        Lang::Id => (
            "Craft 1",
            "Craft 5",
            "Craft 10",
            format!("Craft Max ({max_batch})"),
            "Craft Custom",
            "Kembali",
        ),
    };
    vec![
        CreateActionRow::Buttons(vec![
            btn("craftbatch", owner, &format!("{recipe_id}:1"), c1, ButtonStyle::Success)
                .disabled(max_batch < 1),
            btn("craftbatch", owner, &format!("{recipe_id}:5"), c5, ButtonStyle::Success)
                .disabled(max_batch < 5),
            btn("craftbatch", owner, &format!("{recipe_id}:10"), c10, ButtonStyle::Success)
                .disabled(max_batch < 10),
            btn("craftbatch", owner, &format!("{recipe_id}:max"), &cmax, ButtonStyle::Primary)
                .disabled(max_batch < 1),
            btn("craftcustom", owner, recipe_id, ccust, ButtonStyle::Primary)
                .disabled(max_batch < 1),
        ]),
        CreateActionRow::Buttons(vec![btn("craft", owner, "", back, ButtonStyle::Secondary)]),
    ]
}

pub fn craft_custom_modal(
    owner: u64,
    recipe_id: &str,
    recipe_label: &str,
    max_batch: i64,
    lang: Lang,
) -> CreateModal {
    let (label, ph, title) = match lang {
        Lang::En => (
            format!("Batch qty (max {max_batch})"),
            "e.g. 15 or max",
            format!("Craft Custom — {recipe_label}"),
        ),
        Lang::Id => (
            format!("Jumlah batch (maks {max_batch})"),
            "Contoh: 15 atau max",
            format!("Craft Custom — {recipe_label}"),
        ),
    };
    let label: String = label.chars().take(45).collect();
    let title: String = title.chars().take(45).collect();
    let input = CreateInputText::new(InputTextStyle::Short, label, "qty")
        .placeholder(ph)
        .required(true)
        .min_length(1)
        .max_length(10);
    CreateModal::new(format!("{PREFIX}:craftmodal:{owner}:{recipe_id}"), title)
        .components(vec![CreateActionRow::InputText(input)])
}

pub fn order_select(
    owner: u64,
    orders: &[(i64, String)],
    lang: Lang,
) -> Vec<CreateActionRow> {
    let options: Vec<CreateSelectMenuOption> = orders
        .iter()
        .take(25)
        .map(|(id, label)| CreateSelectMenuOption::new(label.clone(), id.to_string()))
        .collect();
    let (ph, back) = match lang {
        Lang::En => ("Select an NPC order to fulfill", "Back"),
        Lang::Id => ("Pilih pesanan NPC untuk dipenuhi", "Kembali"),
    };
    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                format!("{PREFIX}:orderfulfill:{owner}"),
                CreateSelectMenuKind::String { options },
            )
            .placeholder(ph),
        ),
        CreateActionRow::Buttons(vec![btn("craft", owner, "", back, ButtonStyle::Secondary)]),
    ]
}

pub fn crew_menu(owner: u64, has_crews: bool, lang: Lang) -> Vec<CreateActionRow> {
    let (hire, assign, upg, menu) = match lang {
        Lang::En => ("Hire Crew (100 Bac)", "Assign & Automate", "Upgrade Crew", "Main Menu"),
        Lang::Id => ("Rekrut Kru (100 Bac)", "Tugaskan & Otomasi", "Upgrade Kru", "Menu Utama"),
    };
    let mut row1 = vec![btn("crewhire", owner, "", hire, ButtonStyle::Success)];
    if has_crews {
        row1.push(btn("crewassign", owner, "", assign, ButtonStyle::Primary));
        row1.push(btn("crewupg", owner, "", upg, ButtonStyle::Primary));
    }
    vec![
        CreateActionRow::Buttons(row1),
        CreateActionRow::Buttons(vec![
            btn("crew", owner, "", "Refresh", ButtonStyle::Secondary),
            btn("profile", owner, "", menu, ButtonStyle::Secondary),
        ]),
    ]
}

pub fn crew_select(
    owner: u64,
    action: &str,
    crews: &[(i64, String)],
    placeholder: &str,
    lang: Lang,
) -> Vec<CreateActionRow> {
    let options: Vec<CreateSelectMenuOption> = crews
        .iter()
        .take(25)
        .map(|(id, label)| CreateSelectMenuOption::new(label.clone(), id.to_string()))
        .collect();
    let back = match lang {
        Lang::En => "Back",
        Lang::Id => "Kembali",
    };
    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                format!("{PREFIX}:{action}:{owner}"),
                CreateSelectMenuKind::String { options },
            )
            .placeholder(placeholder),
        ),
        CreateActionRow::Buttons(vec![btn("crew", owner, "", back, ButtonStyle::Secondary)]),
    ]
}

pub fn crew_assign_activity(
    owner: u64,
    crew_id: i64,
    stage: CityStage,
    lang: Lang,
) -> Vec<CreateActionRow> {
    let (unassign, back) = match lang {
        Lang::En => ("Unassign", "Back"),
        Lang::Id => ("Lepas Tugas", "Kembali"),
    };
    let auto_row: Vec<CreateButton> = ActivityKind::ALL
        .iter()
        .map(|&kind| {
            let locked = stage < kind.min_stage();
            let a_name = activity_name(kind, lang);
            let label = if locked {
                format!("🔒 {a_name} + Auto")
            } else {
                format!("{a_name} + Auto")
            };
            btn(
                "crewset",
                owner,
                &format!("{crew_id}:{}:1", kind.as_str()),
                &label,
                ButtonStyle::Success,
            )
            .disabled(locked)
        })
        .collect();
    let manual_row: Vec<CreateButton> = ActivityKind::ALL
        .iter()
        .map(|&kind| {
            let locked = stage < kind.min_stage();
            let a_name = activity_name(kind, lang);
            let label = if locked {
                format!("🔒 {a_name} (Manual)")
            } else {
                format!("{a_name} (Manual)")
            };
            btn(
                "crewset",
                owner,
                &format!("{crew_id}:{}:0", kind.as_str()),
                &label,
                ButtonStyle::Primary,
            )
            .disabled(locked)
        })
        .collect();
    vec![
        CreateActionRow::Buttons(auto_row),
        CreateActionRow::Buttons(manual_row),
        CreateActionRow::Buttons(vec![
            btn("crewset", owner, &format!("{crew_id}:none:0"), unassign, ButtonStyle::Danger),
            btn("crew", owner, "", back, ButtonStyle::Secondary),
        ]),
    ]
}

pub fn host_play_button() -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{PREFIX}:play:0"))
            .label("Play")
            .emoji(ReactionType::Unicode("▶️".to_string()))
            .style(ButtonStyle::Success),
    ])]
}
