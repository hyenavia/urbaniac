use crate::application::activities::{ActivityOverview, ClaimSummary, RunStatus};
use crate::application::city::CityOverview;
use crate::application::crafting::CraftOverview;
use crate::application::crew::CrewView;
use crate::application::inventory::{InventoryLine, InventoryView};
use crate::application::market::MarketOverview;
use crate::application::player::ProfileView;
use crate::bot::branding;
use crate::domain::city::{def as project_def, project_cost_bac, ProjectId};
use crate::domain::crafting::RECIPES;
use crate::domain::economy::format_bac;
use crate::domain::player::xp_to_next;
use crate::domain::resources::{find_item, ItemDef};
use rust_decimal::Decimal;
use serenity::all::{CreateEmbed, CreateEmbedFooter};
use std::collections::HashMap;

pub fn help() -> CreateEmbed {
    branding::styled_embed(branding::COLOR_GENERAL)
        .title(format!("{} — {}", branding::NAME, branding::TAGLINE))
        .image(branding::banner_url())
        .description(format!(
            "*{}*\n\
             {} adalah game ekonomi dan pembangunan kota.\n\n\
             `/start` — buat profil dan buka menu\n\
             `/help` — tampilkan bantuan ini\n\n\
             Tombol di menu: **Profil**, **Inventory**, **Aktivitas**, **Kota**, **Jual**, **Klaim Pending**.\n\
             Mulai Farming atau Gathering di menu Aktivitas; aktivitas tetap berjalan saat kamu offline. \
             Setelah selesai, tekan **Ambil Reward**, jual hasilnya ke NPC, lalu bangun proyek di menu **Kota**.",
            branding::SECONDARY_TAGLINE,
            branding::NAME,
        ))
}

pub fn welcome(p: &ProfileView, created: bool) -> CreateEmbed {
    let title = if created {
        format!("Selamat datang di {}!", branding::NAME)
    } else {
        format!("Selamat datang kembali di {}!", branding::NAME)
    };
    profile(p).title(title).image(branding::banner_url())
}

pub fn profile(p: &ProfileView) -> CreateEmbed {
    let pl = &p.player;
    let mut e = branding::styled_embed(branding::COLOR_GENERAL)
        .title(format!("Profil {}", pl.username))
        .field(
            "Level",
            format!("{} ({}/{} XP)", pl.level, pl.xp, xp_to_next(pl.level)),
            true,
        )
        .field("Bac", format_bac(p.balance), true)
        .field("Kota", p.city_stage.name(), true)
        .field(
            "Inventory",
            format!("{}/{} slot", p.inventory.slots_used, p.inventory.slot_capacity),
            true,
        )
        .footer(CreateEmbedFooter::new(format!(
            "Bergabung {} • {}",
            pl.created_at.format("%d-%m-%Y"),
            branding::FOOTER
        )));
    if let Some(url) = &pl.avatar_url {
        e = e.thumbnail(url);
    }
    e
}

fn fmt_lines(lines: &[InventoryLine], empty: &str) -> String {
    if lines.is_empty() {
        return empty.to_string();
    }
    lines
        .iter()
        .map(|l| {
            format!(
                "{} [{}] × {}/{}",
                l.item.name, l.item.rarity, l.quantity, l.item.stack_limit
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn inventory(v: &InventoryView, note: Option<String>) -> CreateEmbed {
    let mut e = branding::styled_embed(branding::COLOR_INVENTORY)
        .title(format!("Inventory ({}/{} slot)", v.slots_used, v.slot_capacity))
        .field("Item", fmt_lines(&v.lines, "Kosong."), false);
    if !v.pending.is_empty() {
        e = e.field("Pending Claim", fmt_lines(&v.pending, ""), false);
    }
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn notice(title: &str, text: &str) -> CreateEmbed {
    branding::styled_embed(branding::COLOR_SYSTEM).title(title).description(text)
}

pub fn activities(o: &ActivityOverview, note: Option<String>) -> CreateEmbed {
    let mut lines: Vec<String> = Vec::new();
    for r in &o.runs {
        match r.status {
            RunStatus::Running => {
                lines.push(format!("{} — selesai <t:{}:R>", r.kind.name(), r.ends_at.timestamp()))
            }
            RunStatus::Ready => lines.push(format!("{} — **siap diambil**", r.kind.name())),
        }
    }
    for _ in (o.runs.len() as i64)..o.slots_total {
        lines.push("(slot kosong)".to_string());
    }
    let progress = o
        .progress
        .iter()
        .map(|p| {
            let extra = if p.state.overflow > 0 {
                format!(" (+{} XP overflow)", p.state.overflow)
            } else {
                String::new()
            };
            format!(
                "{} Lv {} — {}/{} XP, maks Lv {}{}",
                p.kind.name(),
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
        .title(format!("Aktivitas ({}/{} slot)", o.runs.len(), o.slots_total))
        .field("Slot", lines.join("\n"), false)
        .field("Level", progress, false);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn claim_note(sums: &[ClaimSummary]) -> String {
    if sums.is_empty() {
        return "Belum ada reward yang siap diambil.".to_string();
    }
    sums.iter()
        .map(|s| {
            let items = s
                .items
                .iter()
                .map(|(it, stored, pending)| {
                    if *pending > 0 {
                        format!("{} +{} ({} masuk Pending Claim)", it.name, stored, pending)
                    } else {
                        format!("{} +{}", it.name, stored)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let lvl = if s.new_level > s.old_level {
                format!(" — naik ke Lv {}!", s.new_level)
            } else {
                String::new()
            };
            format!("{}: {}; +{} XP{}", s.kind.name(), items, s.xp_gain, lvl)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn sell_prompt(v: &InventoryView, prices: &HashMap<&'static str, Decimal>) -> CreateEmbed {
    let text = v
        .lines
        .iter()
        .map(|l| {
            let price = prices.get(l.item.id).copied().unwrap_or(Decimal::ZERO);
            format!("{} × {} — {} / unit", l.item.name, l.quantity, format_bac(price))
        })
        .collect::<Vec<_>>()
        .join("\n");
    branding::styled_embed(branding::COLOR_ECONOMY)
        .title("Jual ke NPC (Harga Pasar Dinamis)")
        .description(text)
}

pub fn sell_item(item: &ItemDef, owned: i64, unit_price: Decimal) -> CreateEmbed {
    branding::styled_embed(branding::COLOR_ECONOMY)
        .title(format!("Jual {}", item.name))
        .description(format!(
            "Kamu punya **{}**. Harga NPC saat ini: {} / unit.",
            owned,
            format_bac(unit_price)
        ))
}

pub fn market(ov: &MarketOverview, note: Option<String>) -> CreateEmbed {
    let price_lines = ov
        .prices
        .iter()
        .map(|p| {
            let pct = ((p.multiplier - Decimal::ONE) * Decimal::from(100)).round_dp(0);
            let trend = if pct > Decimal::ZERO {
                format!("+{pct}%")
            } else {
                format!("{pct}%")
            };
            format!(
                "**{}** — {} (dasar {}, {trend})",
                p.item.name,
                format_bac(p.current_price),
                format_bac(p.base_price)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let listing_lines = if ov.listings.is_empty() {
        "Belum ada listing aktif.".to_string()
    } else {
        ov.listings
            .iter()
            .map(|l| {
                format!(
                    "#{} **{} × {}** — target {}/unit, fee {}, habis <t:{}:R>",
                    l.id,
                    l.item.name,
                    l.quantity,
                    format_bac(l.target_unit_price),
                    format_bac(l.listing_fee),
                    l.expires_at.timestamp()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mut e = branding::styled_embed(branding::COLOR_ECONOMY)
        .title("Pasar & Makelar (NPC Market)")
        .field("Harga Pasar NPC Saat Ini", price_lines, false)
        .field(
            format!("Listing Makelar Aktif ({}/5)", ov.listings.len()),
            listing_lines,
            false,
        )
        .field("Wallet", format_bac(ov.wallet_balance), true)
        .field("Biaya Makelar", "Fee 2% (min 1 Bac) + Komisi 5% saat terjual", true);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn city(ov: &CityOverview, note: Option<String>) -> CreateEmbed {
    let facilities = if ov.facilities.is_empty() {
        "Belum ada fasilitas.".to_string()
    } else {
        ov.facilities
            .iter()
            .map(|f| format!("{} Lv {}", f.id.name(), f.level))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let running = match &ov.running_project {
        Some(r) => format!(
            "{} — selesai <t:{}:R>",
            r.project.name(),
            r.ends_at.timestamp()
        ),
        None => "Tidak ada proyek berjalan.".to_string(),
    };
    let projects = ProjectId::ALL
        .iter()
        .map(|&pid| {
            let p = project_def(pid);
            let built = ov.facilities.iter().any(|f| f.id == pid);
            if built {
                format!("**{}** — sudah dibangun", p.name)
            } else {
                let items = p
                    .cost_items
                    .iter()
                    .map(|(id, qty)| {
                        let name = find_item(id).map(|i| i.name).unwrap_or(id);
                        format!("{name} x{qty}")
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "**{}** — {} + {} ({} menit)\n{}",
                    p.name,
                    format_bac(project_cost_bac(p)),
                    items,
                    p.duration_secs / 60,
                    p.description
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let mut e = branding::styled_embed(branding::COLOR_INFRASTRUCTURE)
        .title(format!("Kota — {}", ov.stage.name()))
        .field("City Treasury", format_bac(ov.treasury), true)
        .field("Wallet Pribadi", format_bac(ov.wallet_balance), true)
        .field("Fasilitas", facilities, false)
        .field("Proyek Berjalan", running, false)
        .field("Daftar Proyek", projects, false);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn craft(ov: &CraftOverview, note: Option<String>) -> CreateEmbed {
    let running = if ov.running.is_empty() {
        "Tidak ada crafting berjalan.".to_string()
    } else {
        ov.running
            .iter()
            .map(|r| {
                let tag = if r.auto_sell { " [Auto-Sell]" } else { "" };
                format!(
                    "**{} × {}**{} — selesai <t:{}:R>",
                    r.recipe.name,
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
                .map(|(id, q)| {
                    let name = find_item(id).map(|i| i.name).unwrap_or(id);
                    format!("{name} x{q}")
                })
                .collect::<Vec<_>>()
                .join(" + ");
            format!("**{}** [{}] — {} ({}d)", r.name, r.tier.name(), ins, r.duration_secs)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let orders = if ov.orders.is_empty() {
        "Belum ada pesanan aktif.".to_string()
    } else {
        ov.orders
            .iter()
            .map(|o| {
                format!(
                    "#{} **{} × {}** — hadiah {} (+{} Rep XP)",
                    o.id,
                    o.item.name,
                    o.quantity,
                    format_bac(o.reward_bac),
                    o.rep_xp
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mut e = branding::styled_embed(branding::COLOR_ECONOMY)
        .title("Crafting & Pesanan NPC")
        .field(
            "Reputasi Toko",
            format!(
                "Lv {} ({}/{} XP) • Bonus Order +{}% • Selesai: {}",
                ov.rep_level, ov.rep_xp, ov.rep_xp_needed, ov.rep_bonus_pct, ov.completed_orders
            ),
            false,
        )
        .field("Crafting Berjalan (maks 2)", running, false)
        .field("Resep", recipes, false)
        .field("Pesanan NPC Aktif", orders, false);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}

pub fn crew(crews: &[CrewView], note: Option<String>) -> CreateEmbed {
    let list = if crews.is_empty() {
        "Belum ada kru. Rekrut kru untuk bonus Efficiency, Yield, Luck, dan otomasi aktivitas!".to_string()
    } else {
        crews
            .iter()
            .map(|c| {
                let task = match c.assigned {
                    Some(a) if c.auto_enabled => format!("{} [AUTO]", a.name()),
                    Some(a) => a.name().to_string(),
                    None => "Menganggur".to_string(),
                };
                let upg = c
                    .upgrade_cost
                    .map(|u| format!("Upgrade: {}", format_bac(u)))
                    .unwrap_or_else(|| "Maks Lv".to_string());
                format!(
                    "#{} **{}** [{} Lv {}] — Spesialis: {}\nTugas: {} • Eff +{}% / Yield +{}% / Luck +{}% • {}",
                    c.id,
                    c.name,
                    c.rarity,
                    c.level,
                    c.specialty.name(),
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
    let mut e = branding::styled_embed(branding::COLOR_GENERAL)
        .title(format!("Kru & Automasi ({}/4)", crews.len()))
        .field("Daftar Kru", list, false);
    if let Some(n) = note {
        e = e.description(n);
    }
    e
}
