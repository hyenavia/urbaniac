use crate::application::activities::{ActivityOverview, ClaimSummary, RunStatus};
use crate::application::city::CityOverview;
use crate::application::crafting::CraftOverview;
use crate::application::crew::CrewView;
use crate::application::inventory::{InventoryLine, InventoryView};
use crate::application::market::MarketOverview;
use crate::application::player::ProfileView;
use crate::bot::branding;
use crate::domain::city::{def as project_def, project_cost_bac_for_level, ProjectId};
use crate::domain::crafting::RECIPES;
use crate::domain::economy::format_bac;
use crate::domain::i18n::{
    activity_name, item_name, project_desc, project_name, recipe_name, recipe_tier_name,
    season_name, stage_era, stage_name, world_event_name, Lang,
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

pub fn help(lang: Lang) -> CreateEmbed {
    let (title, desc) = match lang {
        Lang::En => (
            "📖 Urbaniac Gameplay Guide",
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
        ),
        Lang::Id => (
            "📖 Panduan Bermain Urbaniac",
            "Ikuti alur bermain berikut dari **Kampung** hingga **Metropolis**:\n\n\
             **1️⃣ Kumpulkan Resource (Aktivitas)**\n\
             Buka **Aktivitas** -> mulai **Farming** / **Gathering** (2 slot). Saat selesai, kamu akan mendapat **notifikasi DM** -> tekan **Ambil Reward**.\n\n\
             **2️⃣ Dapatkan Bac (Jual NPC / Pasar & Makelar)**\n\
             Buka **Jual NPC** untuk langsung menukar item menjadi **Bac** (bisa jual 1, 10, Semua, atau Custom), atau titip di **Pasar & Makelar** untuk target harga lebih tinggi.\n\n\
             **3️⃣ Bangun & Naikkan Stage Kota**\n\
             Buka **Kota** -> bangun **Sumur** & **Lumbung** (+4 slot tas/level) -> bangun **Balai Kampung** untuk naik ke **Desa** (membuka **Fishing** & menaikkan cap level ke Lv 20).\n\n\
             **4️⃣ Olah Item & Pesanan (Workshop)**\n\
             Buka **Crafting & Pesanan** -> olah bahan mentah (`Tepung`, `Papan`, `Roti`) -> penuhi **Pesanan NPC** untuk Bac ekstra & Reputasi Toko.\n\n\
             **5️⃣ Otomasi dengan Kru**\n\
             Buka **Kru** -> rekrut kru (100 Bac) -> tugaskan ke aktivitas dengan mode **AUTO** agar berjalan otomatis.",
        ),
    };
    branding::styled_embed(branding::COLOR_EXPLORATION)
        .title(title)
        .description(desc)
}

pub fn host_panel() -> CreateEmbed {
    branding::styled_embed(branding::COLOR_SYSTEM)
        .title("🎮  Play Urbaniac")
        .description(
            "Urbaniac is an RPG economy & city-building game played directly inside Discord.\n\n\
             ▶️  Click **Play** below to open your personal **Private Thread**.\n\
             💾  Your progress, **Bac**, city, and items are saved automatically.\n\
             🏗️  Grow your settlement from a **Hamlet** into a **Metropolis**.\n\
             🔔  Completion alerts for activities, projects, and crafting are sent straight to your **DMs**.\n\
             🌐  You can switch between **English** and **Bahasa Indonesia** anytime inside your thread.",
        )
        .image(branding::banner_url())
}

pub fn welcome(p: &ProfileView, created: bool, lang: Lang) -> CreateEmbed {
    let title = match (created, lang) {
        (true, Lang::En) => format!("🌱 Welcome, {}!", p.player.username),
        (true, Lang::Id) => format!("🌱 Selamat Datang, {}!", p.player.username),
        (false, _) => format!("👤 {}", p.player.username.to_uppercase()),
    };
    profile(p, lang).title(title)
}

pub fn profile(p: &ProfileView, lang: Lang) -> CreateEmbed {
    let pl = &p.player;
    let needed = xp_to_next(pl.level);
    let s_name = stage_name(p.city_stage, lang);
    let s_era = stage_era(p.city_stage, lang);
    let (res_label, prog_label, econ_label, inv_label, loc_label, slots_word) = match lang {
        Lang::En => (
            "Resident",
            "Character Progression",
            "Economy",
            "Inventory",
            "Current Location",
            "Slots",
        ),
        Lang::Id => (
            "Warga",
            "Progres Karakter",
            "Ekonomi",
            "Inventory",
            "Lokasi Saat Ini",
            "Slot",
        ),
    };
    let mut e = branding::styled_embed(branding::COLOR_GENERAL)
        .title(format!("👤 {}", pl.username.to_uppercase()))
        .description(format!("**LV. {:02}** · {} {}", pl.level, s_name, res_label))
        .field(prog_label, xp_progress_bar(pl.xp, needed), false)
        .field(econ_label, format!("🪙 {}", format_bac(p.balance)), true)
        .field(
            inv_label,
            format!("🎒 {}/{} {}", p.inventory.slots_used, p.inventory.slot_capacity, slots_word),
            true,
        )
        .field(loc_label, format!("🏘️ {} · {}", s_name, s_era), true);
    if let Some(url) = &pl.avatar_url {
        e = e.thumbnail(url);
    }
    e
}

fn fmt_lines(lines: &[InventoryLine], empty: &str, lang: Lang) -> String {
    if lines.is_empty() {
        return empty.to_string();
    }
    lines
        .iter()
        .map(|l| {
            format!(
                "{} **{}** × {} `/{}` · *{}*",
                item_icon(l.item.id),
                item_name(l.item.id, lang),
                l.quantity,
                l.item.stack_limit,
                l.item.rarity
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn inventory(v: &InventoryView, note: Option<String>, lang: Lang) -> CreateEmbed {
    let (title, stored_hdr, empty_msg, pending_hdr) = match lang {
        Lang::En => (
            format!("🎒 Inventory — {}/{} Slots", v.slots_used, v.slot_capacity),
            "Stored Items",
            "Your bag is empty.",
            "⏳ Pending Claim (Bag Full)",
        ),
        Lang::Id => (
            format!("🎒 Inventory — {}/{} Slot", v.slots_used, v.slot_capacity),
            "Item Tersimpan",
            "Tas masih kosong.",
            "⏳ Pending Claim (Tas Penuh)",
        ),
    };
    let mut e = branding::styled_embed(branding::COLOR_INVENTORY)
        .title(title)
        .field(stored_hdr, fmt_lines(&v.lines, empty_msg, lang), false);
    if !v.pending.is_empty() {
        e = e.field(pending_hdr, fmt_lines(&v.pending, "", lang), false);
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
            RunStatus::Running => lines.push(match lang {
                Lang::En => format!("⏳ **{}** — finishes <t:{}:R>", a_name, r.ends_at.timestamp()),
                Lang::Id => format!("⏳ **{}** — selesai <t:{}:R>", a_name, r.ends_at.timestamp()),
            }),
            RunStatus::Ready => lines.push(match lang {
                Lang::En => format!("✅ **{}** — **Ready to Collect**", a_name),
                Lang::Id => format!("✅ **{}** — **Siap Diambil**", a_name),
            }),
        }
    }
    let empty_slot = match lang {
        Lang::En => "▫ *(empty slot)*",
        Lang::Id => "▫ *(slot kosong)*",
    };
    for _ in (o.runs.len() as i64)..o.slots_total {
        lines.push(empty_slot.to_string());
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
    let (title, slots_hdr, levels_hdr) = match lang {
        Lang::En => (
            format!("⚡ Activity Board ({}/{} Slots)", o.runs.len(), o.slots_total),
            "Activity Slots",
            "Activity Levels",
        ),
        Lang::Id => (
            format!("⚡ Activity Board ({}/{} Slot)", o.runs.len(), o.slots_total),
            "Slot Aktivitas",
            "Level Aktivitas",
        ),
    };
    let mut e = branding::styled_embed(branding::COLOR_ACTIVITIES)
        .title(title)
        .field(slots_hdr, lines.join("\n"), false)
        .field(levels_hdr, progress, false);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn claim_note(sums: &[ClaimSummary], lang: Lang) -> String {
    if sums.is_empty() {
        return match lang {
            Lang::En => "No rewards are ready to collect yet.".to_string(),
            Lang::Id => "Belum ada reward yang siap diambil.".to_string(),
        };
    }
    sums.iter()
        .map(|s| {
            let items = s
                .items
                .iter()
                .map(|(it, stored, pending)| {
                    let name = item_name(it.id, lang);
                    if *pending > 0 {
                        match lang {
                            Lang::En => format!(
                                "{} {} +{} ({} went to Pending Claim)",
                                item_icon(it.id),
                                name,
                                stored,
                                pending
                            ),
                            Lang::Id => format!(
                                "{} {} +{} ({} masuk Pending Claim)",
                                item_icon(it.id),
                                name,
                                stored,
                                pending
                            ),
                        }
                    } else {
                        format!("{} {} +{}", item_icon(it.id), name, stored)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let lvl = if s.new_level > s.old_level {
                match lang {
                    Lang::En => format!(" — leveled up to **LV. {:02}**!", s.new_level),
                    Lang::Id => format!(" — naik ke **LV. {:02}**!", s.new_level),
                }
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
    let title = match lang {
        Lang::En => "🪙 Sell to NPC (Dynamic Market Prices)",
        Lang::Id => "🪙 Jual ke NPC (Harga Pasar Dinamis)",
    };
    branding::styled_embed(branding::COLOR_ECONOMY)
        .title(title)
        .description(text)
}

pub fn sell_item(item: &ItemDef, owned: i64, unit_price: Decimal, lang: Lang) -> CreateEmbed {
    let name = item_name(item.id, lang);
    let (title, desc) = match lang {
        Lang::En => (
            format!("{} Sell {}", item_icon(item.id), name),
            format!(
                "You own **{}**.\nCurrent NPC price: **{} / unit**.",
                owned,
                format_bac(unit_price)
            ),
        ),
        Lang::Id => (
            format!("{} Jual {}", item_icon(item.id), name),
            format!(
                "Kamu punya **{}**.\nHarga NPC saat ini: **{} / unit**.",
                owned,
                format_bac(unit_price)
            ),
        ),
    };
    branding::styled_embed(branding::COLOR_ECONOMY)
        .title(title)
        .description(desc)
}

pub fn market(ov: &MarketOverview, note: Option<String>, lang: Lang) -> CreateEmbed {
    let base_word = match lang {
        Lang::En => "base",
        Lang::Id => "dasar",
    };
    let price_lines = ov
        .prices
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
        .join("\n");

    let listing_lines = if ov.listings.is_empty() {
        match lang {
            Lang::En => "No active listings yet.".to_string(),
            Lang::Id => "Belum ada listing aktif.".to_string(),
        }
    } else {
        ov.listings
            .iter()
            .map(|l| match lang {
                Lang::En => format!(
                    "#{} {} **{} × {}** — target {}/unit, fee {}, expires <t:{}:R>",
                    l.id,
                    item_icon(l.item.id),
                    item_name(l.item.id, lang),
                    l.quantity,
                    format_bac(l.target_unit_price),
                    format_bac(l.listing_fee),
                    l.expires_at.timestamp()
                ),
                Lang::Id => format!(
                    "#{} {} **{} × {}** — target {}/unit, fee {}, habis <t:{}:R>",
                    l.id,
                    item_icon(l.item.id),
                    item_name(l.item.id, lang),
                    l.quantity,
                    format_bac(l.target_unit_price),
                    format_bac(l.listing_fee),
                    l.expires_at.timestamp()
                ),
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let none_word = match lang {
        Lang::En => "None",
        Lang::Id => "Tidak ada",
    };
    let trend_label = ov
        .calendar
        .trend_item_id
        .map(|id| item_name(id, lang))
        .unwrap_or(none_word);
    let event_label = ov
        .calendar
        .active_event
        .map(|e| world_event_name(e, lang))
        .unwrap_or(none_word);
    let cal_info = match lang {
        Lang::En => format!(
            "Day **{}** · {} • Event: {} • Trend: {}",
            ov.calendar.day,
            season_name(ov.calendar.season, lang),
            event_label,
            trend_label
        ),
        Lang::Id => format!(
            "Hari **{}** · {} • Event: {} • Tren: {}",
            ov.calendar.day,
            season_name(ov.calendar.season, lang),
            event_label,
            trend_label
        ),
    };

    let (title, cal_hdr, price_hdr, list_hdr, fee_hdr, fee_val) = match lang {
        Lang::En => (
            "🏪 Market & Broker (NPC Market)",
            "World Calendar",
            "Current NPC Market Prices",
            format!("Active Broker Listings ({}/5)", ov.listings.len()),
            "Broker Fees",
            "2% Fee (min 1 Bac) + 5% Commission",
        ),
        Lang::Id => (
            "🏪 Pasar & Makelar (NPC Market)",
            "Kalender Global",
            "Harga Pasar NPC Saat Ini",
            format!("Listing Makelar Aktif ({}/5)", ov.listings.len()),
            "Biaya Makelar",
            "Fee 2% (min 1 Bac) + Komisi 5%",
        ),
    };

    let mut e = branding::styled_embed(branding::COLOR_ECONOMY)
        .title(title)
        .field(cal_hdr, cal_info, false)
        .field(price_hdr, price_lines, false)
        .field(list_hdr, listing_lines, false)
        .field("Wallet", format!("🪙 {}", format_bac(ov.wallet_balance)), true)
        .field(fee_hdr, fee_val, true);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn city(ov: &CityOverview, note: Option<String>, lang: Lang) -> CreateEmbed {
    let facilities = if ov.facilities.is_empty() {
        match lang {
            Lang::En => "No facilities built yet.".to_string(),
            Lang::Id => "Belum ada fasilitas.".to_string(),
        }
    } else {
        ov.facilities
            .iter()
            .map(|f| format!("🏛️ **{}** `LV. {:02}`", project_name(f.id, lang), f.level))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let running = match &ov.running_project {
        Some(r) => match lang {
            Lang::En => format!(
                "🚧 **{}** — finishes <t:{}:R>",
                project_name(r.project, lang),
                r.ends_at.timestamp()
            ),
            Lang::Id => format!(
                "🚧 **{}** — selesai <t:{}:R>",
                project_name(r.project, lang),
                r.ends_at.timestamp()
            ),
        },
        None => match lang {
            Lang::En => "No active project.".to_string(),
            Lang::Id => "Tidak ada proyek berjalan.".to_string(),
        },
    };
    let min_word = match lang {
        Lang::En => "min",
        Lang::Id => "menit",
    };
    let max_word = match lang {
        Lang::En => "Max",
        Lang::Id => "Maks",
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
                return Some(format!("✅ **{}** — Lv {} ({max_word})", p_name, cur_lvl));
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
                "🔨 **{}** (Lv {}) — {} + {} ({} {min_word})\n*{}*",
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

    let (title, wallet_hdr, econ_hdr, econ_val, fac_hdr, run_hdr, avail_hdr) = match lang {
        Lang::En => (
            format!("🗺️ City — {} ({})", stage_name(ov.stage, lang), stage_era(ov.stage, lang)),
            "Personal Wallet",
            "City Economy",
            format!(
                "Tax: **{}%** • Welfare: **{}%** • Effectiveness: **{}%** • Mod: **{}x**",
                ov.tax_rate_pct, ov.welfare, ov.effectiveness_pct, ov.local_modifier
            ),
            "Installed Facilities",
            "Active Project",
            "Available Projects",
        ),
        Lang::Id => (
            format!("🗺️ Kota — {} ({})", stage_name(ov.stage, lang), stage_era(ov.stage, lang)),
            "Wallet Pribadi",
            "Ekonomi Kota",
            format!(
                "Pajak: **{}%** • Welfare: **{}%** • Efektivitas: **{}%** • Mod: **{}x**",
                ov.tax_rate_pct, ov.welfare, ov.effectiveness_pct, ov.local_modifier
            ),
            "Fasilitas Terpasang",
            "Proyek Berjalan",
            "Proyek Tersedia",
        ),
    };

    let mut e = branding::styled_embed(branding::COLOR_INFRASTRUCTURE)
        .title(title)
        .field("City Treasury", format!("🏦 {}", format_bac(ov.treasury)), true)
        .field(wallet_hdr, format!("🪙 {}", format_bac(ov.wallet_balance)), true)
        .field(econ_hdr, econ_val, false)
        .field(fac_hdr, facilities, false)
        .field(run_hdr, running, false)
        .field(avail_hdr, projects, false);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn craft(ov: &CraftOverview, note: Option<String>, lang: Lang) -> CreateEmbed {
    let running = if ov.running.is_empty() {
        match lang {
            Lang::En => "No crafting in progress.".to_string(),
            Lang::Id => "Tidak ada crafting berjalan.".to_string(),
        }
    } else {
        ov.running
            .iter()
            .map(|r| {
                let tag = if r.auto_sell { " [Auto-Sell]" } else { "" };
                match lang {
                    Lang::En => format!(
                        "⏳ **{} × {}**{} — finishes <t:{}:R>",
                        recipe_name(r.recipe.id, lang),
                        r.quantity,
                        tag,
                        r.ends_at.timestamp()
                    ),
                    Lang::Id => format!(
                        "⏳ **{} × {}**{} — selesai <t:{}:R>",
                        recipe_name(r.recipe.id, lang),
                        r.quantity,
                        tag,
                        r.ends_at.timestamp()
                    ),
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let sec_unit = match lang {
        Lang::En => "s",
        Lang::Id => "d",
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
                "{} **{}** `[{}]` — {} ({}{sec_unit})",
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
        match lang {
            Lang::En => "No active orders.".to_string(),
            Lang::Id => "Belum ada pesanan aktif.".to_string(),
        }
    } else {
        ov.orders
            .iter()
            .map(|o| {
                let reward_word = match lang {
                    Lang::En => "reward",
                    Lang::Id => "hadiah",
                };
                format!(
                    "#{} {} **{} × {}** — {reward_word} **{}** (+{} Rep XP)",
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

    let (title, rep_hdr, rep_val, run_hdr, rec_hdr, ord_hdr) = match lang {
        Lang::En => (
            "⚒️ Workshop & NPC Orders",
            "Shop Reputation",
            format!(
                "`LV. {:02}` ({}/{} XP) • Order Bonus +{}% • Completed: {}",
                ov.rep_level, ov.rep_xp, ov.rep_xp_needed, ov.rep_bonus_pct, ov.completed_orders
            ),
            "Active Crafting (max 2)",
            "Recipes",
            "Active NPC Orders",
        ),
        Lang::Id => (
            "⚒️ Workshop & Pesanan NPC",
            "Reputasi Toko",
            format!(
                "`LV. {:02}` ({}/{} XP) • Bonus Order +{}% • Selesai: {}",
                ov.rep_level, ov.rep_xp, ov.rep_xp_needed, ov.rep_bonus_pct, ov.completed_orders
            ),
            "Crafting Berjalan (maks 2)",
            "Resep",
            "Pesanan NPC Aktif",
        ),
    };

    let mut e = branding::styled_embed(branding::COLOR_ECONOMY)
        .title(title)
        .field(rep_hdr, rep_val, false)
        .field(run_hdr, running, false)
        .field(rec_hdr, recipes, false)
        .field(ord_hdr, orders, false);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn crew(crews: &[CrewView], note: Option<String>, lang: Lang) -> CreateEmbed {
    let list = if crews.is_empty() {
        match lang {
            Lang::En => "No crew members yet. Hire crew for Efficiency, Yield, Luck bonuses and activity automation!".to_string(),
            Lang::Id => "Belum ada kru. Rekrut kru untuk bonus Efficiency, Yield, Luck, dan otomasi aktivitas!".to_string(),
        }
    } else {
        crews
            .iter()
            .map(|c| {
                let task = match c.assigned {
                    Some(a) if c.auto_enabled => format!("{} `[AUTO]`", activity_name(a, lang)),
                    Some(a) => activity_name(a, lang).to_string(),
                    None => match lang {
                        Lang::En => "Idle".to_string(),
                        Lang::Id => "Menganggur".to_string(),
                    },
                };
                let upg = c
                    .upgrade_cost
                    .map(|u| format!("Upgrade: {}", format_bac(u)))
                    .unwrap_or_else(|| match lang {
                        Lang::En => "Max Lv".to_string(),
                        Lang::Id => "Maks Lv".to_string(),
                    });
                match lang {
                    Lang::En => format!(
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
                    ),
                    Lang::Id => format!(
                        "#{} **{}** `[{} LV. {:02}]` — Spesialis: {}\nTugas: {} • Eff +{}% / Yield +{}% / Luck +{}% • {}",
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
                    ),
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let (title, hdr) = match lang {
        Lang::En => (format!("👥 Crew & Automation ({}/4)", crews.len()), "Crew Roster"),
        Lang::Id => (format!("👥 Kru & Automasi ({}/4)", crews.len()), "Daftar Kru"),
    };
    let mut e = branding::styled_embed(branding::COLOR_SOCIAL)
        .title(title)
        .field(hdr, list, false);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}
