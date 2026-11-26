use crate::application::inventory::{InventoryLine, InventoryView};
use crate::application::player::ProfileView;
use crate::domain::economy::format_bac;
use crate::domain::player::xp_to_next;
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
             Tombol di menu: **Profil**, **Inventory**, **Klaim Pending**.\n\
             Aktivitas (Farming, Gathering), pasar, dan proyek kota menyusul di tahap berikutnya.",
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
