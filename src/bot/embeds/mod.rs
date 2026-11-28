use crate::application::activities::{ActivityOverview, ClaimSummary, RunStatus};
use crate::application::city::CityOverview;
use crate::application::crafting::CraftOverview;
use crate::application::crew::CrewView;
use crate::application::inventory::{InventoryLine, InventoryView};
use crate::application::market::{MarketOverview, MarketPriceView};
use crate::application::player::ProfileView;
use crate::bot::branding;
use crate::domain::city::{def as project_def, project_cost_bac_for_level, CityStage, ProjectId};
use crate::domain::crafting::{batch_duration_secs, RecipeDef, RECIPES};
use crate::domain::economy::format_bac;
use crate::domain::i18n::{
    activity_name, event_bonus_hint, item_name, project_desc, project_name, recipe_name,
    recipe_tier_name, season_bonus_hint, season_name, stage_era, stage_name, world_event_name,
    Lang,
};
use crate::domain::player::xp_to_next;
use crate::domain::resources::ItemDef;
use rust_decimal::Decimal;
use serenity::all::CreateEmbed;
use std::collections::HashMap;

pub fn item_icon(id: &str) -> &'static str {
    match id {
        "wheat" | "flour" => "🌾",
        "potato" => "🥔",
        "wood" | "plank" => "🪵",
        "berries" => "🫐",
        "wild_herb" => "🌿",
        "herb_extract" => "🧪",
        "bread" => "🍞",
        "wooden_crate" => "📦",
        "healing_tonic" => "⚗️",
        "river_fish" => "🐟",
        "silver_carp" => "🐠",
        "golden_eel" => "🐍",
        "stone" => "🪨",
        "iron_ore" => "⛏️",
        "amethyst" => "💎",
        "star_crystal" => "✨",
        _ => "🔹",
    }
}

/// Membuat Unicode progress bar 20 karakter + persentase.
pub fn xp_progress_bar(xp: i64, needed: i64) -> String {
    let needed = needed.max(1);
    let ratio = (xp.max(0) as f64 / needed as f64).clamp(0.0, 1.0);
    let filled = (ratio * 20.0).round() as usize;
    let empty = 20usize.saturating_sub(filled);
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
    format!("`{bar}`\n{xp} / {needed} XP ({:.1}%)", ratio * 100.0)
}

pub fn help(_lang: Lang) -> CreateEmbed {
    branding::styled_embed(branding::COLOR_EXPLORATION)
        .title("📖 Urbaniac Gameplay Guide")
        .description(
            "Follow this progression path from **Hamlet** to **Metropolis**:\n\n\
             **1️⃣ Gather Resources (Activities)**\n\
             Open **Activities** -> start **Farming** / **Gathering** (2 slots). When done, you'll get a **DM alert** -> press **Collect Rewards**.\n\n\
             **2️⃣ Earn Bac (Sell NPC / Market & Broker)**\n\
             Open **Sell NPC** to trade items directly for **Bac** (sell 1, 10, All, or Custom), or consign to **Market & Broker** for higher target prices.\n\n\
             **3️⃣ Build & Upgrade Your City**\n\
             Open **City** -> build **Well** & **Granary** (+4 bag slots/level) -> build **Village Hall** to reach **Village** (unlocks **Fishing** & Lv 20 cap).\n\n\
             **4️⃣ Craft Items & Orders (Workshop)**\n\
             Open **Crafting & Orders** -> process raw materials (`Wheat Flour`, `Wood Plank`, `Village Bread`) -> fulfill **NPC Orders** for bonus Bac & Shop Reputation.\n\n\
             **5️⃣ Automate with Crew**\n\
             Open **Crew** -> hire crew (100 Bac) -> assign them to activities in **AUTO** mode so they run continuously.",
        )
}

pub fn host_panel() -> CreateEmbed {
    branding::styled_embed(branding::COLOR_SYSTEM)
        .title("🎮  Play Urbaniac")
        .description(
            "Urbaniac is an RPG economy & city-building game played directly inside Discord.\n\n\
             ▶️  Click **Play** to open your personal **Private Thread**.\n\
             🚀  Click **Launch** to open the **3D City Activity** directly.\n\
             💾  Your progress, **Bac**, city, and items are saved automatically.\n\
             🏗️  Grow your settlement from a **Hamlet** into a **Metropolis**.\n\
             🔔  Completion alerts for activities, projects, and crafting are sent straight to your **DMs**.",
        )
        .image(branding::banner_url())
}

pub fn welcome(p: &ProfileView, created: bool, lang: Lang) -> CreateEmbed {
    let title = if created {
        format!("🌱 Welcome, {}!", p.player.username)
    } else {
        format!("👤 {}", p.player.username.to_uppercase())
    };
    profile(p, lang).title(title)
}

fn next_objective(stage: CityStage, facilities: &[(ProjectId, i32)], _lang: Lang) -> &'static str {
    let has = |id: ProjectId| facilities.iter().any(|(fid, lvl)| *fid == id && *lvl > 0);
    match stage {
        CityStage::Kampung => {
            if !has(ProjectId::Well) {
                "Gather Wood & build **Well** in **City** (+10% Farming & Gathering yield)."
            } else if !has(ProjectId::Granary) {
                "Build **Granary** in **City** (+4 bag slots & +3% activity yield)."
            } else {
                "Build **Village Hall** to advance to **Village** (unlocks Fishing & 3rd Activity Slot)."
            }
        }
        CityStage::Desa => {
            if !has(ProjectId::MarketHall) {
                "Craft Planks & Bread, then build **Market Hall** (+5% NPC sell price)."
            } else {
                "Build **Town Hall** to advance to **Town** (unlocks Mining & 4th Activity Slot)."
            }
        }
        CityStage::KotaKecil => {
            "Mine Iron Ore & build **Trade Port** (3rd Craft Slot & +15% NPC Order rewards)."
        }
        CityStage::KotaDagang => {
            "Gather Amethyst & Herbal Tonic, then build **Metropolis Tower** (+15% Activity & Craft speed)."
        }
        CityStage::Metropolis => {
            "Max out Granary Lv 5, Crew levels, Activity levels (Lv 50), and Shop Reputation!"
        }
    }
}

pub fn profile(p: &ProfileView, lang: Lang) -> CreateEmbed {
    let pl = &p.player;
    let needed = xp_to_next(pl.level);
    let s_name = stage_name(p.city_stage, lang);
    let s_era = stage_era(p.city_stage, lang);
    let pending_note = if p.inventory.pending.is_empty() {
        String::new()
    } else {
        format!(" (⏳ {} pending)", p.inventory.pending.len())
    };
    let mut e = branding::styled_embed(branding::COLOR_GENERAL)
        .title(format!("👤 {}", pl.username.to_uppercase()))
        .description(format!("**LV. {:02}** · {} Resident", pl.level, s_name))
        .field("Character Progression", xp_progress_bar(pl.xp, needed), false)
        .field("Economy", format!("🪙 {}", format_bac(p.balance)), true)
        .field(
            "Inventory",
            format!(
                "🎒 {}/{} Slots{}",
                p.inventory.slots_used, p.inventory.slot_capacity, pending_note
            ),
            true,
        )
        .field("Current Location", format!("🏘️ {} · {}", s_name, s_era), true)
        .field("🎯 Next Objective", next_objective(p.city_stage, &p.facilities, lang), false);
    if let Some(url) = &pl.avatar_url {
        e = e.thumbnail(url);
    }
    e
}

pub fn with_voice_boost(embed: CreateEmbed, boosted: bool) -> CreateEmbed {
    if boosted {
        embed.field(
            "🔊 Voice Boost",
            "Active (+20% Activity & Craft Speed • +15% XP)",
            false,
        )
    } else {
        embed
    }
}

fn fmt_lines(lines: &[InventoryLine], empty: &str, lang: Lang) -> String {
    if lines.is_empty() {
        return empty.to_string();
    }
    lines
        .iter()
        .map(|l| {
            format!(
                "{} **{}** × {} · *{}*",
                item_icon(l.item.id),
                item_name(l.item.id, lang),
                l.quantity,
                l.item.rarity
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn inventory(v: &InventoryView, note: Option<String>, lang: Lang) -> CreateEmbed {
    let mut e = branding::styled_embed(branding::COLOR_INVENTORY)
        .title(format!("🎒 Inventory — {}/{} Slots", v.slots_used, v.slot_capacity))
        .field("Stored Items", fmt_lines(&v.lines, "Your bag is empty.", lang), false);
    if !v.pending.is_empty() {
        e = e.field("⏳ Pending Claim (Bag Full)", fmt_lines(&v.pending, "", lang), false);
    }
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn notice(title: &str, text: &str) -> CreateEmbed {
    branding::styled_embed(branding::COLOR_SYSTEM)
        .title(title)
        .description(text)
}

pub fn activities(o: &ActivityOverview, note: Option<String>, lang: Lang) -> CreateEmbed {
    let mut lines: Vec<String> = Vec::new();
    for r in &o.runs {
        let a_name = activity_name(r.kind, lang);
        match r.status {
            RunStatus::Running => {
                lines.push(format!("⏳ **{}** — finishes <t:{}:R>", a_name, r.ends_at.timestamp()));
            }
            RunStatus::Ready => {
                lines.push(format!("✅ **{}** — **Ready to Collect**", a_name));
            }
        }
    }
    for _ in (o.runs.len() as i64)..o.slots_total {
        lines.push("▫ *(empty slot)*".to_string());
    }
    let progress = o
        .progress
        .iter()
        .map(|p| {
            let extra = if p.state.overflow > 0 {
                format!(" (+{} overflow)", p.state.overflow)
            } else {
                String::new()
            };
            format!(
                "**{}** `LV. {:02}` — {}/{} XP (Cap Lv {}){}",
                activity_name(p.kind, lang),
                p.state.level,
                p.state.xp,
                p.xp_needed,
                p.cap,
                extra
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut e = branding::styled_embed(branding::COLOR_ACTIVITIES)
        .title(format!("⚡ Activity Board ({}/{} Slots)", o.runs.len(), o.slots_total))
        .field("Activity Slots", lines.join("\n"), false)
        .field("Activity Levels", progress, false);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn claim_note(sums: &[ClaimSummary], lang: Lang) -> String {
    if sums.is_empty() {
        return "No rewards are ready to collect yet.".to_string();
    }
    sums.iter()
        .map(|s| {
            let items = s
                .items
                .iter()
                .map(|(it, stored, pending)| {
                    let name = item_name(it.id, lang);
                    if *pending > 0 {
                        format!(
                            "{} {} +{} ({} went to Pending Claim)",
                            item_icon(it.id),
                            name,
                            stored,
                            pending
                        )
                    } else {
                        format!("{} {} +{}", item_icon(it.id), name, stored)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let lvl = if s.new_level > s.old_level {
                format!(" — leveled up to **LV. {:02}**!", s.new_level)
            } else {
                String::new()
            };
            format!("**{}**: {}; +{} XP{}", activity_name(s.kind, lang), items, s.xp_gain, lvl)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn sell_prompt(
    v: &InventoryView,
    prices: &HashMap<&'static str, Decimal>,
    lang: Lang,
) -> CreateEmbed {
    let text = v
        .lines
        .iter()
        .map(|l| {
            let price = prices.get(l.item.id).copied().unwrap_or(Decimal::ZERO);
            format!(
                "{} **{}** × {} — {} / unit",
                item_icon(l.item.id),
                item_name(l.item.id, lang),
                l.quantity,
                format_bac(price)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    branding::styled_embed(branding::COLOR_ECONOMY)
        .title("🪙 Sell to NPC (Dynamic Market Prices)")
        .description(text)
}

pub fn sell_item(item: &ItemDef, owned: i64, unit_price: Decimal, lang: Lang) -> CreateEmbed {
    let name = item_name(item.id, lang);
    branding::styled_embed(branding::COLOR_ECONOMY)
        .title(format!("{} Sell {}", item_icon(item.id), name))
        .description(format!(
            "You own **{}**.\nCurrent NPC price: **{} / unit**.",
            owned,
            format_bac(unit_price)
        ))
}

fn format_price_lines(prices: &[&MarketPriceView], base_word: &str, lang: Lang) -> String {
    if prices.is_empty() {
        return "-".to_string();
    }
    prices
        .iter()
        .map(|p| {
            let pct = ((p.multiplier - Decimal::ONE) * Decimal::from(100)).round_dp(0);
            let trend = if pct > Decimal::ZERO {
                format!("📈 +{pct}%")
            } else if pct < Decimal::ZERO {
                format!("📉 {pct}%")
            } else {
                "➖ 0%".to_string()
            };
            format!(
                "{} **{}** — {} *({base_word} {}, {trend})*",
                item_icon(p.item.id),
                item_name(p.item.id, lang),
                format_bac(p.current_price),
                format_bac(p.base_price)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn split_market_price_fields(prices: &[MarketPriceView], lang: Lang) -> (String, String) {
    let is_crafted = |id: &str| RECIPES.iter().any(|r| r.output_item_id == id);
    let raw: Vec<&MarketPriceView> = prices.iter().filter(|p| !is_crafted(p.item.id)).collect();
    let crafted: Vec<&MarketPriceView> = prices.iter().filter(|p| is_crafted(p.item.id)).collect();
    (
        format_price_lines(&raw, "base", lang),
        format_price_lines(&crafted, "base", lang),
    )
}

pub fn market(ov: &MarketOverview, note: Option<String>, lang: Lang) -> CreateEmbed {
    let (raw_lines, crafted_lines) = split_market_price_fields(&ov.prices, lang);

    let listing_lines = if ov.listings.is_empty() {
        "No active listings yet.".to_string()
    } else {
        ov.listings
            .iter()
            .map(|l| {
                format!(
                    "#{} {} **{} × {}** — target {}/unit, fee {}, expires <t:{}:R>",
                    l.id,
                    item_icon(l.item.id),
                    item_name(l.item.id, lang),
                    l.quantity,
                    format_bac(l.target_unit_price),
                    format_bac(l.listing_fee),
                    l.expires_at.timestamp()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let trend_label = ov
        .calendar
        .trend_item_id
        .map(|id| item_name(id, lang))
        .unwrap_or("None");
    let season_hint = season_bonus_hint(ov.calendar.season, lang);
    let event_label = ov
        .calendar
        .active_event
        .map(|e| format!("{} ({})", world_event_name(e, lang), event_bonus_hint(e, lang)))
        .unwrap_or_else(|| "None".to_string());
    let cal_info = format!(
        "Day **{}** (1 day = 2h) · **{}** *({season_hint})*\nEvent: {} • Trend (+15%): **{}**",
        ov.calendar.day,
        season_name(ov.calendar.season, lang),
        event_label,
        trend_label
    );

    let mut e = branding::styled_embed(branding::COLOR_ECONOMY)
        .title("🏪 Market & Broker (NPC Market)")
        .field("World Calendar", cal_info, false)
        .field("Raw & Gathering Prices", raw_lines, false)
        .field("Crafted Goods Prices", crafted_lines, false)
        .field(format!("Active Broker Listings ({}/5)", ov.listings.len()), listing_lines, false)
        .field("Wallet", format!("🪙 {}", format_bac(ov.wallet_balance)), true)
        .field("Broker Fees", "1% Fee (min 0.50 Bac) + 3% Commission", true);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn city(ov: &CityOverview, note: Option<String>, lang: Lang) -> CreateEmbed {
    let facilities = if ov.facilities.is_empty() {
        "No facilities built yet.".to_string()
    } else {
        ov.facilities
            .iter()
            .map(|f| format!("🏛️ **{}** `LV. {:02}`", project_name(f.id, lang), f.level))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let running = match &ov.running_project {
        Some(r) => format!(
            "🚧 **{}** — finishes <t:{}:R>",
            project_name(r.project, lang),
            r.ends_at.timestamp()
        ),
        None => "No active project.".to_string(),
    };
    let projects = ProjectId::ALL
        .iter()
        .filter_map(|&pid| {
            let p = project_def(pid);
            let p_name = project_name(pid, lang);
            let p_desc = project_desc(pid, lang);
            let cur_lvl = ov
                .facilities
                .iter()
                .find(|f| f.id == pid)
                .map(|f| f.level)
                .unwrap_or(0);
            if cur_lvl >= p.max_level {
                return Some(format!("✅ **{}** — Lv {} (Max)", p_name, cur_lvl));
            }
            if ov.stage < p.min_stage {
                return None;
            }
            let next_lvl = cur_lvl + 1;
            let items = p
                .cost_items
                .iter()
                .map(|(id, qty)| {
                    format!("{} {} x{}", item_icon(id), item_name(id, lang), qty * (next_lvl as i64))
                })
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!(
                "🔨 **{}** (Lv {}) — {} + {} ({} min)\n*{}*",
                p_name,
                next_lvl,
                format_bac(project_cost_bac_for_level(p, next_lvl)),
                items,
                p.duration_secs / 60,
                p_desc
            ))
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let net_cashflow = ov.est_tax_income - ov.est_expense;
    let net_str = if net_cashflow >= Decimal::ZERO {
        format!("+{}", format_bac(net_cashflow))
    } else {
        format_bac(net_cashflow)
    };
    let yield_str = if ov.yield_bonus_pct >= 0 {
        format!("+{}%", ov.yield_bonus_pct)
    } else {
        format!("{}%", ov.yield_bonus_pct)
    };

    let econ_val = format!(
        "Tax: **{}%** • Welfare: **{}%** • Activity Yield Bonus: **{}**\n\
         Cashflow / 30m: +{} (tax) − {} (upkeep, {} active crew) = **{}**\n\
         *5%/10% keeps Welfare 100%; 15% boosts tax but lowers Welfare to 80%. Projects spend Treasury first.*",
        ov.tax_rate_pct,
        ov.welfare,
        yield_str,
        format_bac(ov.est_tax_income),
        format_bac(ov.est_expense),
        ov.assigned_crews,
        net_str,
    );

    let mut e = branding::styled_embed(branding::COLOR_INFRASTRUCTURE)
        .title(format!("🗺️ City — {} ({})", stage_name(ov.stage, lang), stage_era(ov.stage, lang)))
        .field("City Treasury", format!("🏦 {}", format_bac(ov.treasury)), true)
        .field("Personal Wallet", format!("🪙 {}", format_bac(ov.wallet_balance)), true)
        .field("City Economy", econ_val, false)
        .field("Installed Facilities", facilities, false)
        .field("Active Project", running, false)
        .field("Available Projects", projects, false);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn craft(ov: &CraftOverview, note: Option<String>, lang: Lang) -> CreateEmbed {
    let running = if ov.running.is_empty() {
        "No crafting in progress.".to_string()
    } else {
        ov.running
            .iter()
            .map(|r| {
                let tag = if r.auto_sell { " [Auto-Sell]" } else { "" };
                format!(
                    "⏳ **{} × {}**{} — finishes <t:{}:R>",
                    recipe_name(r.recipe.id, lang),
                    r.quantity,
                    tag,
                    r.ends_at.timestamp()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let recipes = RECIPES
        .iter()
        .map(|r| {
            let ins = r
                .inputs
                .iter()
                .map(|(id, q)| format!("{} {} x{}", item_icon(id), item_name(id, lang), q))
                .collect::<Vec<_>>()
                .join(" + ");
            format!(
                "{} **{}** `[{}]` — {} ({}s)",
                item_icon(r.output_item_id),
                recipe_name(r.id, lang),
                recipe_tier_name(r.tier, lang),
                ins,
                r.duration_secs
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let orders = if ov.orders.is_empty() {
        "No active orders.".to_string()
    } else {
        ov.orders
            .iter()
            .map(|o| {
                format!(
                    "#{} {} **{} × {}** — reward **{}** (+{} Rep XP)",
                    o.id,
                    item_icon(o.item.id),
                    item_name(o.item.id, lang),
                    o.quantity,
                    format_bac(o.reward_bac),
                    o.rep_xp
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let rep_val = format!(
        "`LV. {:02}` ({}/{} XP) • Order Bonus +{}% • Completed: {}",
        ov.rep_level, ov.rep_xp, ov.rep_xp_needed, ov.rep_bonus_pct, ov.completed_orders
    );

    let mut e = branding::styled_embed(branding::COLOR_ECONOMY)
        .title("⚒️ Workshop & NPC Orders")
        .field("Shop Reputation", rep_val, false)
        .field(format!("Active Crafting (max {})", ov.max_concurrent), running, false)
        .field("Recipes", recipes, false)
        .field("Active NPC Orders", orders, false);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn crew(crews: &[CrewView], note: Option<String>, lang: Lang) -> CreateEmbed {
    let list = if crews.is_empty() {
        "No crew members yet. Hire crew for Efficiency, Yield, Luck bonuses and activity automation!"
            .to_string()
    } else {
        crews
            .iter()
            .map(|c| {
                let task = match c.assigned {
                    Some(a) if c.auto_enabled => format!("{} `[AUTO]`", activity_name(a, lang)),
                    Some(a) => activity_name(a, lang).to_string(),
                    None => "Idle".to_string(),
                };
                let upg = c
                    .upgrade_cost
                    .map(|u| format!("Upgrade: {}", format_bac(u)))
                    .unwrap_or_else(|| "Max Lv".to_string());
                format!(
                    "#{} **{}** `[{} LV. {:02}]` — Specialist: {}\nTask: {} • Eff +{}% / Yield +{}% / Luck +{}% • {}",
                    c.id,
                    c.name,
                    c.rarity,
                    c.level,
                    activity_name(c.specialty, lang),
                    task,
                    c.bonus.efficiency_pct,
                    c.bonus.yield_pct,
                    c.bonus.luck_pct,
                    upg
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let mut e = branding::styled_embed(branding::COLOR_SOCIAL)
        .title(format!("👥 Crew & Automation ({}/4)", crews.len()))
        .field("Crew Roster", list, false);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::economy::npc_buy_price;
    use crate::domain::resources::ITEMS;

    #[test]
    fn market_price_fields_stay_under_discord_limit() {
        let mult = Decimal::new(15_000, 4);
        let views: Vec<MarketPriceView> = ITEMS
            .iter()
            .map(|item| {
                let base = npc_buy_price(item);
                MarketPriceView {
                    item,
                    base_price: base,
                    current_price: (base * mult).round_dp(2),
                    multiplier: mult,
                }
            })
            .collect();

        let (raw, crafted) = split_market_price_fields(&views, Lang::En);
        assert!(raw.len() <= 1024, "raw price field too long ({}): {}", raw.len(), raw);
        assert!(
            crafted.len() <= 1024,
            "crafted price field too long ({}): {}",
            crafted.len(),
            crafted
        );
    }
}

pub fn craft_item(
    recipe: &RecipeDef,
    stock: &HashMap<&str, i64>,
    max_batch: i64,
    lang: Lang,
) -> CreateEmbed {
    let r_name = recipe_name(recipe.id, lang);
    let tier = recipe_tier_name(recipe.tier, lang);
    let inputs = recipe
        .inputs
        .iter()
        .map(|(id, need)| {
            let have = stock.get(id).copied().unwrap_or(0);
            format!(
                "{} **{}**: {} / {} per unit",
                item_icon(id),
                item_name(id, lang),
                have,
                need
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let d1 = batch_duration_secs(recipe.duration_secs, 1);
    let d5 = batch_duration_secs(recipe.duration_secs, 5);
    let d10 = batch_duration_secs(recipe.duration_secs, 10);

    branding::styled_embed(branding::COLOR_ECONOMY)
        .title(format!("{} Craft {} [{}]", item_icon(recipe.output_item_id), r_name, tier))
        .description(format!("Max craftable right now: **{max_batch}**"))
        .field("Required Materials (Owned / Need)", inputs, false)
        .field(
            "Batch Craft Duration",
            format!("×1: **{d1}s** • ×5: **{d5}s** • ×10: **{d10}s**"),
            false,
        )
}
