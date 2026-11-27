use crate::application::economy::{self, SaleOutcome, SellError, SellQty};
use crate::application::{activities, city, crafting, inventory, market, player};
use crate::bot::{commands, components, embeds, AppState};
use crate::domain::activities::ActivityKind;
use crate::domain::city::ProjectId;
use crate::domain::economy::format_bac;
use crate::domain::market::{calc_listing_fee, ListingDuration};
use crate::domain::resources::find_item;
use rust_decimal::Decimal;
use serenity::all::*;
use std::collections::HashMap;

pub async fn handle_command(
    ctx: &Context,
    cmd: &CommandInteraction,
    app: &AppState,
) -> anyhow::Result<()> {
    match cmd.data.name.as_str() {
        "start" => commands::start::run(ctx, cmd, app).await,
        "help" => commands::help::run(ctx, cmd, app).await,
        other => {
            tracing::warn!(command = other, "command tidak dikenal");
            Ok(())
        }
    }
}

async fn deny(ctx: &Context, c: &ComponentInteraction, text: &str) -> anyhow::Result<()> {
    let msg = CreateInteractionResponseMessage::new().content(text).ephemeral(true);
    c.create_response(&ctx.http, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}

async fn update(
    ctx: &Context,
    c: &ComponentInteraction,
    embed: CreateEmbed,
    rows: Vec<CreateActionRow>,
) -> anyhow::Result<()> {
    let msg = CreateInteractionResponseMessage::new().embed(embed).components(rows);
    c.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}

async fn show_activities(
    ctx: &Context,
    c: &ComponentInteraction,
    app: &AppState,
    id: i64,
    note: Option<String>,
) -> anyhow::Result<()> {
    let ov = activities::overview(&app.pool, id).await?;
    update(ctx, c, embeds::activities(&ov, note), components::activity_menu(id as u64)).await
}

async fn show_city(
    ctx: &Context,
    c: &ComponentInteraction,
    app: &AppState,
    id: i64,
    note: Option<String>,
) -> anyhow::Result<()> {
    let ov = city::overview(&app.pool, id).await?;
    update(ctx, c, embeds::city(&ov, note), components::city_menu(id as u64)).await
}

async fn show_market(
    ctx: &Context,
    c: &ComponentInteraction,
    app: &AppState,
    id: i64,
    note: Option<String>,
) -> anyhow::Result<()> {
    let ov = market::overview(&app.pool, id).await?;
    let has_listings = !ov.listings.is_empty();
    update(
        ctx,
        c,
        embeds::market(&ov, note),
        components::market_menu(id as u64, has_listings),
    )
    .await
}

async fn show_craft(
    ctx: &Context,
    c: &ComponentInteraction,
    app: &AppState,
    id: i64,
    note: Option<String>,
) -> anyhow::Result<()> {
    let ov = crafting::overview(&app.pool, id).await?;
    let auto_sell = ov.auto_sell;
    update(
        ctx,
        c,
        embeds::craft(&ov, note),
        components::craft_menu(id as u64, auto_sell),
    )
    .await
}

pub async fn handle_component(
    ctx: &Context,
    c: &ComponentInteraction,
    app: &AppState,
) -> anyhow::Result<()> {
    let Some((action, owner, rest)) = components::parse_custom_id(&c.data.custom_id) else {
        return Ok(()); // bukan komponen milik kita
    };
    // Validasi pemilik data.
    if c.user.id.get() != owner {
        return deny(ctx, c, "Menu ini milik pemain lain. Jalankan /start untuk menu milikmu.").await;
    }
    let id = owner as i64;
    let Some(profile) = player::get_player_profile(&app.pool, id).await? else {
        return deny(ctx, c, "Profil belum ada. Jalankan /start.").await;
    };

    match action {
        "profile" => update(ctx, c, embeds::profile(&profile), components::main_menu(owner)).await,
        "inventory" => {
            let v = inventory::view(&app.pool, id).await?;
            update(ctx, c, embeds::inventory(&v, None), components::main_menu(owner)).await
        }
        "claim" => {
            let results = inventory::claim_pending(&app.pool, id).await?;
            let note = if results.is_empty() {
                "Tidak ada Pending Claim.".to_string()
            } else {
                results
                    .iter()
                    .map(|r| {
                        let name = find_item(&r.item_id).map(|i| i.name).unwrap_or("?");
                        if r.remaining == 0 {
                            format!("{name}: {} diklaim", r.claimed)
                        } else {
                            format!("{name}: {} diklaim, {} masih pending (inventory penuh)", r.claimed, r.remaining)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let v = inventory::view(&app.pool, id).await?;
            update(ctx, c, embeds::inventory(&v, Some(note)), components::main_menu(owner)).await
        }
        "activities" => show_activities(ctx, c, app, id, None).await,
        "startact" => {
            let Some(kind) = ActivityKind::parse(rest) else { return Ok(()) };
            let note = match activities::start(&app.pool, id, kind, app.activity_duration_pct).await {
                Ok(run) => {
                    app.notify.notify_one(); // bangunkan scheduler
                    format!("{} dimulai — selesai <t:{}:R>.", kind.name(), run.ends_at.timestamp())
                }
                Err(activities::ActivityError::Db(e)) => return Err(e.into()),
                Err(e) => e.to_string(),
            };
            show_activities(ctx, c, app, id, Some(note)).await
        }
        "collect" => {
            let sums = activities::collect_ready(&app.pool, id).await?;
            show_activities(ctx, c, app, id, Some(embeds::claim_note(&sums))).await
        }
        "city" => show_city(ctx, c, app, id, None).await,
        "cityproj" => {
            let Some(proj) = ProjectId::parse(rest) else { return Ok(()) };
            let key = format!("cityproj:{}", c.id.get());
            let note = match city::start_project(
                &app.pool,
                id,
                proj,
                app.activity_duration_pct,
                &key,
            )
            .await
            {
                Ok(run) => {
                    app.notify.notify_one();
                    format!(
                        "Proyek {} dimulai — selesai <t:{}:R>.",
                        proj.name(),
                        run.ends_at.timestamp()
                    )
                }
                Err(city::CityError::Db(e)) => return Err(e.into()),
                Err(city::CityError::Wallet(e)) => return Err(e.into()),
                Err(e) => e.to_string(),
            };
            show_city(ctx, c, app, id, Some(note)).await
        }
        "citydep" => {
            let Ok(bac) = rest.parse::<i64>() else { return Ok(()) };
            let amount = Decimal::new(bac * 100, 2);
            let key = format!("citydep:{}", c.id.get());
            let note = match city::deposit_to_treasury(&app.pool, id, amount, &key).await {
                Ok((wallet, treasury)) => format!(
                    "Berhasil setor {} ke City Treasury. Treasury: {}, Wallet: {}.",
                    format_bac(amount),
                    format_bac(treasury),
                    format_bac(wallet)
                ),
                Err(city::CityError::Db(e)) => return Err(e.into()),
                Err(city::CityError::Wallet(e)) => return Err(e.into()),
                Err(e) => e.to_string(),
            };
            show_city(ctx, c, app, id, Some(note)).await
        }
        "sell" => {
            let v = inventory::view(&app.pool, id).await?;
            if v.lines.is_empty() {
                return update(
                    ctx,
                    c,
                    embeds::notice("Jual ke NPC", "Inventory kosong, belum ada yang bisa dijual."),
                    components::main_menu(owner),
                )
                .await;
            }
            let ov = market::overview(&app.pool, id).await?;
            let price_map: HashMap<&'static str, Decimal> =
                ov.prices.iter().map(|p| (p.item.id, p.current_price)).collect();
            let items: Vec<(String, String, i64)> = v
                .lines
                .iter()
                .map(|l| (l.item.name.to_string(), l.item.id.to_string(), l.quantity))
                .collect();
            update(
                ctx,
                c,
                embeds::sell_prompt(&v, &price_map),
                components::sell_select(owner, &items),
            )
            .await
        }
        "sellpick" => {
            let picked = match &c.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => values.first().cloned(),
                _ => None,
            };
            let Some(item) = picked.as_deref().and_then(find_item) else { return Ok(()) };
            let owned = inventory::view(&app.pool, id)
                .await?
                .lines
                .iter()
                .find(|l| l.item.id == item.id)
                .map(|l| l.quantity)
                .unwrap_or(0);
            let unit_price = market::current_npc_price(&app.pool, item).await?;
            update(
                ctx,
                c,
                embeds::sell_item(item, owned, unit_price),
                components::sell_qty(owner, item.id),
            )
            .await
        }
        "market" => show_market(ctx, c, app, id, None).await,
        "mktnew" => {
            let v = inventory::view(&app.pool, id).await?;
            if v.lines.is_empty() {
                return show_market(
                    ctx,
                    c,
                    app,
                    id,
                    Some("Inventory kosong, tidak ada item untuk dititipkan ke Makelar.".to_string()),
                )
                .await;
            }
            let items: Vec<(String, String, i64)> = v
                .lines
                .iter()
                .map(|l| (l.item.name.to_string(), l.item.id.to_string(), l.quantity))
                .collect();
            update(
                ctx,
                c,
                embeds::notice("Titip ke Makelar", "Pilih item dari inventory yang ingin dititipkan."),
                components::broker_item_select(owner, &items),
            )
            .await
        }
        "mktpick" => {
            let picked = match &c.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => values.first().cloned(),
                _ => None,
            };
            let Some(item) = picked.as_deref().and_then(find_item) else { return Ok(()) };
            let owned = inventory::view(&app.pool, id)
                .await?
                .lines
                .iter()
                .find(|l| l.item.id == item.id)
                .map(|l| l.quantity)
                .unwrap_or(0);
            let cur_price = market::current_npc_price(&app.pool, item).await?;
            update(
                ctx,
                c,
                embeds::notice(
                    &format!("Titip {} ke Makelar", item.name),
                    &format!(
                        "Kamu punya **{}**. Harga pasar saat ini: {}/unit.\nPilih jumlah yang ingin dititipkan:",
                        owned,
                        format_bac(cur_price)
                    ),
                ),
                components::broker_qty_select(owner, item.id, owned),
            )
            .await
        }
        "mktqty" => {
            let Some((item_id, qty_s)) = rest.split_once(':') else { return Ok(()) };
            let Some(item) = find_item(item_id) else { return Ok(()) };
            let Ok(qty) = qty_s.parse::<i64>() else { return Ok(()) };
            let cur_price = market::current_npc_price(&app.pool, item).await?;
            let p110 = (cur_price * Decimal::new(110, 2)).round_dp(2);
            let p125 = (cur_price * Decimal::new(125, 2)).round_dp(2);
            let p140 = (cur_price * Decimal::new(140, 2)).round_dp(2);
            let desc = format!(
                "Item: **{} × {}**\nHarga pasar sekarang: {}/unit\n\n\
                 • **+10% (1 Jam)**: target {}/unit (fee {})\n\
                 • **+25% (6 Jam)**: target {}/unit (fee {})\n\
                 • **+40% (24 Jam)**: target {}/unit (fee {})",
                item.name,
                qty,
                format_bac(cur_price),
                format_bac(p110),
                format_bac(calc_listing_fee(p110, qty)),
                format_bac(p125),
                format_bac(calc_listing_fee(p125, qty)),
                format_bac(p140),
                format_bac(calc_listing_fee(p140, qty)),
            );
            update(
                ctx,
                c,
                embeds::notice("Pilih Target Harga & Durasi", &desc),
                components::broker_terms_select(owner, item.id, qty),
            )
            .await
        }
        "mktpost" => {
            let parts: Vec<&str> = rest.splitn(4, ':').collect();
            if parts.len() != 4 {
                return Ok(());
            }
            let Some(item) = find_item(parts[0]) else { return Ok(()) };
            let Ok(qty) = parts[1].parse::<i64>() else { return Ok(()) };
            let Ok(pct) = parts[2].parse::<i64>() else { return Ok(()) };
            let Some(dur) = ListingDuration::parse(parts[3]) else { return Ok(()) };

            let cur_price = market::current_npc_price(&app.pool, item).await?;
            let target = (cur_price * Decimal::new(pct, 2)).round_dp(2);
            let key = format!("mktpost:{}", c.id.get());
            let note = match market::create_listing(
                &app.pool,
                id,
                item.id,
                qty,
                target,
                dur,
                app.activity_duration_pct,
                &key,
            )
            .await
            {
                Ok(l) => {
                    app.notify.notify_one();
                    format!(
                        "Listing #{} dibuat: **{} × {}** target {}/unit (fee {}, habis <t:{}:R>).",
                        l.id,
                        l.item.name,
                        l.quantity,
                        format_bac(l.target_unit_price),
                        format_bac(l.listing_fee),
                        l.expires_at.timestamp()
                    )
                }
                Err(market::MarketError::Db(e)) => return Err(e.into()),
                Err(market::MarketError::Wallet(e)) => return Err(e.into()),
                Err(e) => e.to_string(),
            };
            show_market(ctx, c, app, id, Some(note)).await
        }
        "craft" => show_craft(ctx, c, app, id, None).await,
        "craftauto" => {
            let on = crafting::toggle_auto_sell(&app.pool, id).await?;
            let note = if on {
                "Auto-Sell hasil craft diaktifkan."
            } else {
                "Auto-Sell hasil craft dimatikan."
            };
            show_craft(ctx, c, app, id, Some(note.to_string())).await
        }
        "craftdo" => {
            let note = match crafting::start_craft(&app.pool, id, rest, app.activity_duration_pct).await {
                Ok(run) => {
                    app.notify.notify_one();
                    format!(
                        "Crafting **{} × {}** dimulai — selesai <t:{}:R>.",
                        run.recipe.name,
                        run.quantity,
                        run.ends_at.timestamp()
                    )
                }
                Err(crafting::CraftError::Db(e)) => return Err(e.into()),
                Err(crafting::CraftError::Wallet(e)) => return Err(e.into()),
                Err(e) => e.to_string(),
            };
            show_craft(ctx, c, app, id, Some(note)).await
        }
        "orderpick" => {
            let ov = crafting::overview(&app.pool, id).await?;
            if ov.orders.is_empty() {
                return show_craft(ctx, c, app, id, Some("Tidak ada pesanan aktif.".to_string())).await;
            }
            let items: Vec<(i64, String)> = ov
                .orders
                .iter()
                .map(|o| {
                    (
                        o.id,
                        format!("#{} {} × {} ({})", o.id, o.item.name, o.quantity, format_bac(o.reward_bac)),
                    )
                })
                .collect();
            update(
                ctx,
                c,
                embeds::notice("Penuhi Pesanan NPC", "Pilih pesanan yang ingin diserahkan."),
                components::order_select(owner, &items),
            )
            .await
        }
        "orderfulfill" => {
            let picked = match &c.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => values.first().cloned(),
                _ => None,
            };
            let Some(order_id) = picked.and_then(|s| s.parse::<i64>().ok()) else { return Ok(()) };
            let note = match crafting::fulfill_order(&app.pool, id, order_id).await {
                Ok(s) => {
                    let lvl = if s.new_rep_level > s.old_rep_level {
                        format!(" — Reputasi naik ke Lv {}!", s.new_rep_level)
                    } else {
                        String::new()
                    };
                    format!(
                        "Pesanan #{} (**{} × {}**) selesai! Hadiah {} + {} Rep XP{}.",
                        order_id,
                        s.item.name,
                        s.quantity,
                        format_bac(s.reward_bac),
                        s.rep_xp_gain,
                        lvl
                    )
                }
                Err(crafting::CraftError::Db(e)) => return Err(e.into()),
                Err(crafting::CraftError::Wallet(e)) => return Err(e.into()),
                Err(e) => e.to_string(),
            };
            show_craft(ctx, c, app, id, Some(note)).await
        }
        "mktcancel" => {
            let ov = market::overview(&app.pool, id).await?;
            if ov.listings.is_empty() {
                return show_market(ctx, c, app, id, Some("Tidak ada listing aktif.".to_string())).await;
            }
            let items: Vec<(i64, String)> = ov
                .listings
                .iter()
                .map(|l| {
                    (
                        l.id,
                        format!("#{} {} × {} ({}/unit)", l.id, l.item.name, l.quantity, format_bac(l.target_unit_price)),
                    )
                })
                .collect();
            update(
                ctx,
                c,
                embeds::notice("Batalkan Listing", "Pilih listing yang ingin dibatalkan (fee tidak dikembalikan)."),
                components::broker_cancel_select(owner, &items),
            )
            .await
        }
        "mktcancelpick" => {
            let picked = match &c.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => values.first().cloned(),
                _ => None,
            };
            let Some(listing_id) = picked.and_then(|s| s.parse::<i64>().ok()) else { return Ok(()) };
            let note = match market::cancel_listing(&app.pool, id, listing_id).await {
                Ok((item, stored, pending)) if pending > 0 => format!(
                    "Listing #{} dibatalkan. {} +{} kembali ke inventory, {} masuk Pending Claim.",
                    listing_id, item.name, stored, pending
                ),
                Ok((item, stored, _)) => format!(
                    "Listing #{} dibatalkan. {} +{} dikembalikan ke inventory.",
                    listing_id, item.name, stored
                ),
                Err(market::MarketError::Db(e)) => return Err(e.into()),
                Err(market::MarketError::Wallet(e)) => return Err(e.into()),
                Err(e) => e.to_string(),
            };
            show_market(ctx, c, app, id, Some(note)).await
        }
        "sellqty" => {
            let Some((item_id, qty_s)) = rest.split_once(':') else { return Ok(()) };
            let qty = if qty_s == "all" {
                SellQty::All
            } else {
                match qty_s.parse::<i64>() {
                    Ok(n) => SellQty::Exact(n),
                    Err(_) => return Ok(()),
                }
            };
            // ID interaksi Discord unik per klik, sehingga aman dari pemrosesan ganda.
            let key = format!("sell:{}", c.id.get());
            let text = match economy::sell_to_npc(&app.pool, id, item_id, qty, &key).await {
                Ok(SaleOutcome::Sold { item, qty, total, balance }) => format!(
                    "Terjual {} {} seharga {}. Saldo: {}.",
                    qty,
                    item.name,
                    format_bac(total),
                    format_bac(balance)
                ),
                Ok(SaleOutcome::Duplicate) => "Penjualan ini sudah diproses.".to_string(),
                Err(SellError::Db(e)) => return Err(e.into()),
                Err(SellError::Wallet(e)) => return Err(e.into()),
                Err(e) => format!("Gagal menjual: {e}"),
            };
            update(ctx, c, embeds::notice("Jual ke NPC", &text), components::main_menu(owner)).await
        }
        _ => Ok(()),
    }
}
