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

pub fn main_menu(owner: u64, _lang: Lang) -> Vec<CreateActionRow> {
    vec![
        CreateActionRow::Buttons(vec![
            btn("profile", owner, "", "Profile", ButtonStyle::Primary),
            btn("inventory", owner, "", "Inventory", ButtonStyle::Secondary),
            btn("activities", owner, "", "Activities", ButtonStyle::Primary),
            btn("city", owner, "", "City", ButtonStyle::Primary),
        ]),
        CreateActionRow::Buttons(vec![
            btn("market", owner, "", "Market & Broker", ButtonStyle::Success),
            btn("craft", owner, "", "Crafting & Orders", ButtonStyle::Success),
            btn("crew", owner, "", "Crew", ButtonStyle::Primary),
            btn("guide", owner, "", "Guide", ButtonStyle::Secondary),
        ]),
    ]
}

pub fn profile_menu(owner: u64, lang: Lang) -> Vec<CreateActionRow> {
    main_menu(owner, lang)
}

pub fn inventory_menu(
    owner: u64,
    has_items: bool,
    has_pending: bool,
    _lang: Lang,
) -> Vec<CreateActionRow> {
    let mut buttons = vec![
        btn("sell", owner, "", "Sell to NPC", ButtonStyle::Success).disabled(!has_items),
    ];
    if has_pending {
        buttons.push(btn("claim", owner, "", "Claim Pending", ButtonStyle::Primary));
    }
    buttons.push(btn("inventory", owner, "", "Refresh", ButtonStyle::Secondary));
    buttons.push(btn("profile", owner, "", "Main Menu", ButtonStyle::Secondary));
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
                format!("Build {p_name}")
            };
            Some(btn("cityproj", owner, pid.as_str(), &label, ButtonStyle::Success))
        })
        .take(5)
        .collect();

    let mut rows = Vec::new();
    if !proj_buttons.is_empty() {
        rows.push(CreateActionRow::Buttons(proj_buttons));
    }
    rows.push(CreateActionRow::Buttons(vec![
        btn("citydepcustom", owner, "", "Deposit", ButtonStyle::Primary),
        btn("citywdcustom", owner, "", "Withdraw", ButtonStyle::Primary),
        btn("citytax", owner, "5", "Tax 5%", ButtonStyle::Secondary),
        btn("citytax", owner, "10", "Tax 10%", ButtonStyle::Secondary),
        btn("citytax", owner, "15", "Tax 15%", ButtonStyle::Secondary),
    ]));
    rows.push(CreateActionRow::Buttons(vec![
        btn("city", owner, "", "Refresh", ButtonStyle::Secondary),
        btn("profile", owner, "", "Main Menu", ButtonStyle::Secondary),
    ]));
    rows
}

pub fn city_deposit_modal(owner: u64, wallet_balance: Decimal, _lang: Lang) -> CreateModal {
    let label: String = format!("Amount (Wallet: {})", format_bac(wallet_balance))
        .chars()
        .take(45)
        .collect();
    let input = CreateInputText::new(InputTextStyle::Short, label, "amount")
        .placeholder("e.g. 100 or max")
        .required(true)
        .min_length(1)
        .max_length(20);
    CreateModal::new(format!("{PREFIX}:citydepmodal:{owner}"), "Deposit to City Treasury")
        .components(vec![CreateActionRow::InputText(input)])
}

pub fn city_withdraw_modal(owner: u64, treasury_balance: Decimal, _lang: Lang) -> CreateModal {
    let label: String = format!("Amount (Treasury: {})", format_bac(treasury_balance))
        .chars()
        .take(45)
        .collect();
    let input = CreateInputText::new(InputTextStyle::Short, label, "amount")
        .placeholder("e.g. 100 or max")
        .required(true)
        .min_length(1)
        .max_length(20);
    CreateModal::new(format!("{PREFIX}:citywdmodal:{owner}"), "Withdraw from City Treasury")
        .components(vec![CreateActionRow::InputText(input)])
}

pub fn activity_menu(owner: u64, stage: CityStage, lang: Lang) -> Vec<CreateActionRow> {
    let act_buttons: Vec<CreateButton> = ActivityKind::ALL
        .iter()
        .map(|&kind| {
            let locked = stage < kind.min_stage();
            let a_name = activity_name(kind, lang);
            let label = if locked {
                format!("🔒 Start {a_name}")
            } else {
                format!("Start {a_name}")
            };
            btn("startact", owner, kind.as_str(), &label, ButtonStyle::Success).disabled(locked)
        })
        .collect();
    vec![
        CreateActionRow::Buttons(act_buttons),
        CreateActionRow::Buttons(vec![
            btn("collect", owner, "", "Collect Rewards", ButtonStyle::Primary),
            btn("activities", owner, "", "Refresh", ButtonStyle::Secondary),
            btn("profile", owner, "", "Main Menu", ButtonStyle::Secondary),
        ]),
    ]
}

/// `items`: (name, item_id, qty)
pub fn sell_select(
    owner: u64,
    items: &[(String, String, i64)],
    _lang: Lang,
) -> Vec<CreateActionRow> {
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
            .placeholder("Select an item to sell"),
        ),
        CreateActionRow::Buttons(vec![btn("profile", owner, "", "Main Menu", ButtonStyle::Secondary)]),
    ]
}

pub fn sell_qty(owner: u64, item_id: &str, _lang: Lang) -> Vec<CreateActionRow> {
    vec![
        CreateActionRow::Buttons(vec![
            btn("sellqty", owner, &format!("{item_id}:1"), "Sell 1", ButtonStyle::Success),
            btn("sellqty", owner, &format!("{item_id}:10"), "Sell 10", ButtonStyle::Success),
            btn("sellqty", owner, &format!("{item_id}:100"), "Sell 100", ButtonStyle::Success),
            btn("sellqty", owner, &format!("{item_id}:all"), "Sell Max", ButtonStyle::Danger),
            btn("sellcustom", owner, item_id, "Sell Custom", ButtonStyle::Primary),
        ]),
        CreateActionRow::Buttons(vec![btn("sell", owner, "", "Back", ButtonStyle::Secondary)]),
    ]
}

pub fn sell_custom_modal(
    owner: u64,
    item_id: &str,
    item_name: &str,
    owned: i64,
    _lang: Lang,
) -> CreateModal {
    let label = format!("Amount of {item_name} (own: {owned})");
    let title = format!("Sell Custom — {item_name}");
    let input = CreateInputText::new(InputTextStyle::Short, label, "qty")
        .placeholder("e.g. 5")
        .required(true)
        .min_length(1)
        .max_length(10);
    CreateModal::new(format!("{PREFIX}:sellmodal:{owner}:{item_id}"), title)
        .components(vec![CreateActionRow::InputText(input)])
}

pub fn market_menu(owner: u64, has_listings: bool, _lang: Lang) -> Vec<CreateActionRow> {
    let mut buttons = vec![
        btn("sell", owner, "", "Sell to NPC", ButtonStyle::Success),
        btn("mktnew", owner, "", "List with Broker", ButtonStyle::Primary),
    ];
    if has_listings {
        buttons.push(btn("mktcancel", owner, "", "Cancel Listing", ButtonStyle::Danger));
    }
    buttons.push(btn("market", owner, "", "Refresh", ButtonStyle::Secondary));
    buttons.push(btn("profile", owner, "", "Main Menu", ButtonStyle::Secondary));
    vec![CreateActionRow::Buttons(buttons)]
}

pub fn broker_item_select(
    owner: u64,
    items: &[(String, String, i64)],
    _lang: Lang,
) -> Vec<CreateActionRow> {
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
            .placeholder("Select an item to consign"),
        ),
        CreateActionRow::Buttons(vec![btn("market", owner, "", "Back", ButtonStyle::Secondary)]),
    ]
}

pub fn broker_qty_select(
    owner: u64,
    item_id: &str,
    owned: i64,
    _lang: Lang,
) -> Vec<CreateActionRow> {
    let mut buttons = Vec::new();
    for q in [1, 5, 10, 25] {
        if owned >= q {
            buttons.push(btn(
                "mktqty",
                owner,
                &format!("{item_id}:{q}"),
                &format!("List {q}"),
                ButtonStyle::Primary,
            ));
        }
    }
    if owned > 0 && !matches!(owned, 1 | 5 | 10 | 25) {
        buttons.push(btn(
            "mktqty",
            owner,
            &format!("{item_id}:{owned}"),
            &format!("All ({owned})"),
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
        "Back",
        ButtonStyle::Secondary,
    )]));
    rows
}

pub fn broker_terms_select(
    owner: u64,
    item_id: &str,
    qty: i64,
    _lang: Lang,
) -> Vec<CreateActionRow> {
    vec![
        CreateActionRow::Buttons(vec![
            btn("mktpost", owner, &format!("{item_id}:{qty}:110:2h"), "+10% (2 Hrs)", ButtonStyle::Success),
            btn("mktpost", owner, &format!("{item_id}:{qty}:120:6h"), "+20% (6 Hrs)", ButtonStyle::Success),
            btn("mktpost", owner, &format!("{item_id}:{qty}:135:24h"), "+35% (24 Hrs)", ButtonStyle::Success),
        ]),
        CreateActionRow::Buttons(vec![btn("market", owner, "", "Back", ButtonStyle::Secondary)]),
    ]
}

pub fn broker_cancel_select(
    owner: u64,
    listings: &[(i64, String)],
    _lang: Lang,
) -> Vec<CreateActionRow> {
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
            .placeholder("Select a listing to cancel"),
        ),
        CreateActionRow::Buttons(vec![btn("market", owner, "", "Back", ButtonStyle::Secondary)]),
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
            btn("orderpick", owner, "", "Fulfill Order", ButtonStyle::Success),
            btn("orderrefresh", owner, "", "Reroll Orders", ButtonStyle::Primary),
            btn("craftauto", owner, "", auto_label, ButtonStyle::Secondary),
            btn("craft", owner, "", "Refresh", ButtonStyle::Secondary),
            btn("profile", owner, "", "Main Menu", ButtonStyle::Secondary),
        ]),
    ]
}

pub fn craft_qty(owner: u64, recipe_id: &str, max_batch: i64, _lang: Lang) -> Vec<CreateActionRow> {
    let cmax = format!("Craft Max ({max_batch})");
    vec![
        CreateActionRow::Buttons(vec![
            btn("craftbatch", owner, &format!("{recipe_id}:1"), "Craft 1", ButtonStyle::Success)
                .disabled(max_batch < 1),
            btn("craftbatch", owner, &format!("{recipe_id}:5"), "Craft 5", ButtonStyle::Success)
                .disabled(max_batch < 5),
            btn("craftbatch", owner, &format!("{recipe_id}:10"), "Craft 10", ButtonStyle::Success)
                .disabled(max_batch < 10),
            btn("craftbatch", owner, &format!("{recipe_id}:max"), &cmax, ButtonStyle::Primary)
                .disabled(max_batch < 1),
            btn("craftcustom", owner, recipe_id, "Craft Custom", ButtonStyle::Primary)
                .disabled(max_batch < 1),
        ]),
        CreateActionRow::Buttons(vec![btn("craft", owner, "", "Back", ButtonStyle::Secondary)]),
    ]
}

pub fn craft_custom_modal(
    owner: u64,
    recipe_id: &str,
    recipe_label: &str,
    max_batch: i64,
    _lang: Lang,
) -> CreateModal {
    let label: String = format!("Batch qty (max {max_batch})")
        .chars()
        .take(45)
        .collect();
    let title: String = format!("Craft Custom — {recipe_label}")
        .chars()
        .take(45)
        .collect();
    let input = CreateInputText::new(InputTextStyle::Short, label, "qty")
        .placeholder("e.g. 15 or max")
        .required(true)
        .min_length(1)
        .max_length(10);
    CreateModal::new(format!("{PREFIX}:craftmodal:{owner}:{recipe_id}"), title)
        .components(vec![CreateActionRow::InputText(input)])
}

pub fn order_select(
    owner: u64,
    orders: &[(i64, String)],
    _lang: Lang,
) -> Vec<CreateActionRow> {
    let options: Vec<CreateSelectMenuOption> = orders
        .iter()
        .take(25)
        .map(|(id, label)| CreateSelectMenuOption::new(label.clone(), id.to_string()))
        .collect();
    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                format!("{PREFIX}:orderfulfill:{owner}"),
                CreateSelectMenuKind::String { options },
            )
            .placeholder("Select an NPC order to fulfill"),
        ),
        CreateActionRow::Buttons(vec![btn("craft", owner, "", "Back", ButtonStyle::Secondary)]),
    ]
}

pub fn crew_menu(owner: u64, has_crews: bool, _lang: Lang) -> Vec<CreateActionRow> {
    let mut row1 = vec![btn("crewhire", owner, "", "Hire Crew (100 Bac)", ButtonStyle::Success)];
    if has_crews {
        row1.push(btn("crewassign", owner, "", "Assign & Automate", ButtonStyle::Primary));
        row1.push(btn("crewupg", owner, "", "Upgrade Crew", ButtonStyle::Primary));
    }
    vec![
        CreateActionRow::Buttons(row1),
        CreateActionRow::Buttons(vec![
            btn("crew", owner, "", "Refresh", ButtonStyle::Secondary),
            btn("profile", owner, "", "Main Menu", ButtonStyle::Secondary),
        ]),
    ]
}

pub fn crew_select(
    owner: u64,
    action: &str,
    crews: &[(i64, String)],
    placeholder: &str,
    _lang: Lang,
) -> Vec<CreateActionRow> {
    let options: Vec<CreateSelectMenuOption> = crews
        .iter()
        .take(25)
        .map(|(id, label)| CreateSelectMenuOption::new(label.clone(), id.to_string()))
        .collect();
    vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                format!("{PREFIX}:{action}:{owner}"),
                CreateSelectMenuKind::String { options },
            )
            .placeholder(placeholder),
        ),
        CreateActionRow::Buttons(vec![btn("crew", owner, "", "Back", ButtonStyle::Secondary)]),
    ]
}

pub fn crew_assign_activity(
    owner: u64,
    crew_id: i64,
    stage: CityStage,
    lang: Lang,
) -> Vec<CreateActionRow> {
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
            btn("crewset", owner, &format!("{crew_id}:none:0"), "Unassign", ButtonStyle::Danger),
            btn("crew", owner, "", "Back", ButtonStyle::Secondary),
        ]),
    ]
}

pub fn host_play_button() -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{PREFIX}:play:0"))
            .label("Play")
            .emoji(ReactionType::Unicode("▶️".to_string()))
            .style(ButtonStyle::Success),
        CreateButton::new(format!("{PREFIX}:launch:0"))
            .label("Launch")
            .emoji(ReactionType::Unicode("🚀".to_string()))
            .style(ButtonStyle::Primary),
    ])]
}

pub fn dm_activity_button() -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{PREFIX}:launch:0"))
            .label("Open Activity")
            .emoji(ReactionType::Unicode("🚀".to_string()))
            .style(ButtonStyle::Primary),
    ])]
}
