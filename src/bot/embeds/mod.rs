use crate::application::activities::{ActivityOverview, ClaimSummary, RunStatus};
use crate::application::city::CityOverview;
use crate::application::inventory::{InventoryLine, InventoryView};
use crate::application::player::ProfileView;
use crate::domain::city::{def as project_def, project_cost_bac, ProjectId};
use crate::domain::economy::{format_bac, npc_buy_price};
use crate::domain::player::xp_to_next;
use crate::domain::resources::{find_item, ItemDef};
use serenity::all::{Colour, CreateEmbed};

const COLOR: u32 = 0x2ecc71;

pub fn help() -> CreateEmbed {
    CreateEmbed::new()
        .title("Urbaniac — Bantuan")
        .colour(Colour::new(COLOR))
        .description(
            "Urbaniac adalah game ekonomi dan pembangunan kota.\n\n\
             `/start` — buat profil dan buka menu\n\
             `/help` — tampilkan bantuan ini\n\n\
             Tombol di menu: **Profil**, **Inventory**, **Aktivitas**, **Kota**, **Jual**, **Klaim Pending**.\n\
             Mulai Farming atau Gathering di menu Aktivitas; aktivitas tetap berjalan saat kamu offline. \
             Setelah selesai, tekan **Ambil Reward**, jual hasilnya ke NPC, lalu bangun proyek di menu **Kota**.",
        )
}

pub fn welcome(p: &ProfileView, created: bool) -> CreateEmbed {
    let title = if created { "Selamat datang di Urbaniac!" } else { "Selamat datang kembali!" };
    profile(p).title(title)
}

pub fn profile(p: &ProfileView) -> CreateEmbed {
    let pl = &p.player;
    let mut e = CreateEmbed::new()
        .title(format!("Profil {}", pl.username))
        .colour(Colour::new(COLOR))
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
        .footer(serenity::all::CreateEmbedFooter::new(format!(
            "Bergabung {}",
            pl.created_at.format("%d-%m-%Y")
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
    let mut e = CreateEmbed::new()
        .title(format!("Inventory ({}/{} slot)", v.slots_used, v.slot_capacity))
        .colour(Colour::new(COLOR))
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
    CreateEmbed::new().title(title).description(text).colour(Colour::new(COLOR))
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
    let mut e = CreateEmbed::new()
        .title(format!("Aktivitas ({}/{} slot)", o.runs.len(), o.slots_total))
        .colour(Colour::new(COLOR))
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

pub fn sell_prompt(v: &InventoryView) -> CreateEmbed {
    let text = v
        .lines
        .iter()
        .map(|l| format!("{} × {} — {} / unit", l.item.name, l.quantity, format_bac(npc_buy_price(l.item))))
        .collect::<Vec<_>>()
        .join("\n");
    CreateEmbed::new()
        .title("Jual ke NPC")
        .description(text)
        .colour(Colour::new(COLOR))
}

pub fn sell_item(item: &ItemDef, owned: i64) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("Jual {}", item.name))
        .description(format!(
            "Kamu punya **{}**. Harga NPC: {} / unit.",
            owned,
            format_bac(npc_buy_price(item))
        ))
        .colour(Colour::new(COLOR))
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

    let mut e = CreateEmbed::new()
        .title(format!("Kota — {}", ov.stage.name()))
        .colour(Colour::new(COLOR))
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
