use crate::application::economy::{self, SaleOutcome, SellError, SellQty};
use crate::application::server::{self, EnterOutcome};
use crate::application::{activities, city, crafting, crew, inventory, market, player};
use crate::bot::{commands, components, embeds, i18n_errors, server_ui, AppState};
use crate::domain::activities::ActivityKind;
use crate::domain::city::ProjectId;
use crate::domain::economy::format_bac;
use crate::domain::i18n::{
    activity_name, item_name, project_name, recipe_name, Lang,
};
use crate::domain::market::{calc_listing_fee, ListingDuration};
use crate::domain::resources::find_item;
use crate::infrastructure::repositories::server_repo::WelcomeConfig;
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
        "host" => commands::host::run(ctx, cmd, app).await,
        "welcome" => commands::welcome::run(ctx, cmd, app).await,
        "announce" => commands::announce::run(ctx, cmd, app).await,
        "giveaway" => commands::giveaway::run_giveaway(ctx, cmd, app).await,
        "cash-giveaway" => commands::giveaway::run_cash_giveaway(ctx, cmd, app).await,
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
    lang: Lang,
    note: Option<String>,
) -> anyhow::Result<()> {
    let ov = activities::overview(&app.pool, id).await?;
    update(
        ctx,
        c,
        embeds::activities(&ov, note, lang),
        components::activity_menu(id as u64, lang),
    )
    .await
}

async fn show_city(
    ctx: &Context,
    c: &ComponentInteraction,
    app: &AppState,
    id: i64,
    lang: Lang,
    note: Option<String>,
) -> anyhow::Result<()> {
    let ov = city::overview(&app.pool, id).await?;
    let built: Vec<(ProjectId, i32)> = ov.facilities.iter().map(|f| (f.id, f.level)).collect();
    update(
        ctx,
        c,
        embeds::city(&ov, note, lang),
        components::city_menu(id as u64, ov.stage, &built, lang),
    )
    .await
}

async fn show_market(
    ctx: &Context,
    c: &ComponentInteraction,
    app: &AppState,
    id: i64,
    lang: Lang,
    note: Option<String>,
) -> anyhow::Result<()> {
    let ov = market::overview(&app.pool, id).await?;
    let has_listings = !ov.listings.is_empty();
    update(
        ctx,
        c,
        embeds::market(&ov, note, lang),
        components::market_menu(id as u64, has_listings, lang),
    )
    .await
}

async fn show_craft(
    ctx: &Context,
    c: &ComponentInteraction,
    app: &AppState,
    id: i64,
    lang: Lang,
    note: Option<String>,
) -> anyhow::Result<()> {
    let ov = crafting::overview(&app.pool, id).await?;
    let auto_sell = ov.auto_sell;
    update(
        ctx,
        c,
        embeds::craft(&ov, note, lang),
        components::craft_menu(id as u64, auto_sell, lang),
    )
    .await
}

async fn show_crew(
    ctx: &Context,
    c: &ComponentInteraction,
    app: &AppState,
    id: i64,
    lang: Lang,
    note: Option<String>,
) -> anyhow::Result<()> {
    let list = crew::list(&app.pool, id).await?;
    let has = !list.is_empty();
    update(
        ctx,
        c,
        embeds::crew(&list, note, lang),
        components::crew_menu(id as u64, has, lang),
    )
    .await
}

async fn handle_play_button(
    ctx: &Context,
    c: &ComponentInteraction,
    app: &AppState,
) -> anyhow::Result<()> {
    if c.guild_id.is_none() {
        return deny(
            ctx,
            c,
            "Use the Play button inside a server channel to create your Private Thread.",
        )
        .await;
    }
    let uid = c.user.id;
    let pid = uid.get() as i64;
    let avatar = c.user.avatar_url();
    let (_, created) = player::register(&app.pool, pid, &c.user.name, avatar.as_deref()).await?;
    let Some(profile) = player::get_player_profile(&app.pool, pid).await? else {
        return deny(ctx, c, "Failed to load your profile.").await;
    };
    let lang = profile.player.language();

    let msg = CreateMessage::new()
        .content(format!("<@{}>", uid.get()))
        .embed(embeds::welcome(&profile, created, lang))
        .components(components::main_menu(uid.get(), lang));

    if let Some(tid) = player::get_thread_id(&app.pool, pid).await? {
        let ch = ChannelId::new(tid);
        if ch.send_message(&ctx.http, msg.clone()).await.is_ok() {
            return deny(ctx, c, &format!("Your private thread is ready: <#{tid}>")).await;
        }
    }

    let thread = c
        .channel_id
        .create_thread(
            &ctx.http,
            CreateThread::new(format!("urbaniac-{}", c.user.name))
                .kind(ChannelType::PrivateThread)
                .invitable(false),
        )
        .await?;
    thread.id.add_thread_member(&ctx.http, uid).await?;
    player::set_thread_id(&app.pool, pid, thread.id.get()).await?;
    thread.id.send_message(&ctx.http, msg).await?;

    deny(
        ctx,
        c,
        &format!("Your private thread has been created: <#{}>", thread.id.get()),
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
    if action == "play" && owner == 0 {
        return handle_play_button(ctx, c, app).await;
    }
    if action == "gwjoin" && owner == 0 {
        let Ok(giveaway_id) = rest.parse::<i64>() else {
            return Ok(());
        };
        return match server::enter_giveaway(&app.pool, giveaway_id, c.user.id.get() as i64).await? {
            EnterOutcome::Entered { giveaway, total_entries } => {
                let msg = CreateInteractionResponseMessage::new()
                    .embed(server_ui::giveaway_active_embed(&giveaway, total_entries))
                    .components(server_ui::giveaway_join_button(giveaway.id, total_entries, false));
                c.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg))
                    .await?;
                let _ = c
                    .create_followup(
                        &ctx.http,
                        CreateInteractionResponseFollowup::new()
                            .content(format!("🎉 You've entered **Giveaway #{}**! Good luck!", giveaway.id))
                            .ephemeral(true),
                    )
                    .await;
                Ok(())
            }
            EnterOutcome::AlreadyEntered { total_entries } => {
                deny(
                    ctx,
                    c,
                    &format!("You are already entered in this giveaway! (Total entries: {total_entries})"),
                )
                .await
            }
            EnterOutcome::NeedsProfile => {
                deny(
                    ctx,
                    c,
                    "This Cash giveaway requires an Urbaniac profile first! Click **Play** or run `/start`, then try again.",
                )
                .await
            }
            EnterOutcome::Ended => deny(ctx, c, "This giveaway has already ended.").await,
            EnterOutcome::NotFound => deny(ctx, c, "Giveaway not found.").await,
        };
    }
    // Validasi pemilik data.
    if c.user.id.get() != owner {
        return deny(
            ctx,
            c,
            "This menu belongs to another player. Run `/start` or click **Play** for your own menu.",
        )
        .await;
    }
    let id = owner as i64;
    let Some(pl) = player::get_player(&app.pool, id).await? else {
        return deny(ctx, c, "No profile yet. Run `/start` or click **Play**.").await;
    };
    let lang = pl.language();

    match action {
        "lang" => {
            let new_lang = player::toggle_lang(&app.pool, id).await?;
            let Some(profile) = player::get_player_profile(&app.pool, id).await? else {
                return deny(
                    ctx,
                    c,
                    match new_lang {
                        Lang::En => "No profile yet. Run `/start`.",
                        Lang::Id => "Profil belum ada. Jalankan `/start`.",
                    },
                )
                .await;
            };
            update(
                ctx,
                c,
                embeds::profile(&profile, new_lang),
                components::main_menu(owner, new_lang),
            )
            .await
        }
        "profile" => {
            let Some(profile) = player::get_player_profile(&app.pool, id).await? else {
                return deny(
                    ctx,
                    c,
                    match lang {
                        Lang::En => "No profile yet. Run `/start`.",
                        Lang::Id => "Profil belum ada. Jalankan `/start`.",
                    },
                )
                .await;
            };
            update(
                ctx,
                c,
                embeds::profile(&profile, lang),
                components::main_menu(owner, lang),
            )
            .await
        }
        "guide" => {
            update(
                ctx,
                c,
                embeds::help(lang),
                components::main_menu(owner, lang),
            )
            .await
        }
        "inventory" => {
            let v = inventory::view(&app.pool, id).await?;
            update(
                ctx,
                c,
                embeds::inventory(&v, None, lang),
                components::main_menu(owner, lang),
            )
            .await
        }
        "claim" => {
            let results = inventory::claim_pending(&app.pool, id).await?;
            let note = if results.is_empty() {
                match lang {
                    Lang::En => "No Pending Claims.".to_string(),
                    Lang::Id => "Tidak ada Pending Claim.".to_string(),
                }
            } else {
                results
                    .iter()
                    .map(|r| {
                        let name = item_name(&r.item_id, lang);
                        if r.remaining == 0 {
                            match lang {
                                Lang::En => format!("{name}: {} claimed", r.claimed),
                                Lang::Id => format!("{name}: {} diklaim", r.claimed),
                            }
                        } else {
                            match lang {
                                Lang::En => format!(
                                    "{name}: {} claimed, {} still pending (inventory full)",
                                    r.claimed, r.remaining
                                ),
                                Lang::Id => format!(
                                    "{name}: {} diklaim, {} masih pending (inventory penuh)",
                                    r.claimed, r.remaining
                                ),
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let v = inventory::view(&app.pool, id).await?;
            update(
                ctx,
                c,
                embeds::inventory(&v, Some(note), lang),
                components::main_menu(owner, lang),
            )
            .await
        }
        "activities" => show_activities(ctx, c, app, id, lang, None).await,
        "startact" => {
            let Some(kind) = ActivityKind::parse(rest) else { return Ok(()) };
            let note = match activities::start(&app.pool, id, kind, app.activity_duration_pct).await {
                Ok(run) => {
                    app.notify.notify_one(); // bangunkan scheduler
                    let a_name = activity_name(kind, lang);
                    match lang {
                        Lang::En => format!("{a_name} started — finishes <t:{}:R>.", run.ends_at.timestamp()),
                        Lang::Id => format!("{a_name} dimulai — selesai <t:{}:R>.", run.ends_at.timestamp()),
                    }
                }
                Err(activities::ActivityError::Db(e)) => return Err(e.into()),
                Err(e) => i18n_errors::activity_error(&e, lang),
            };
            show_activities(ctx, c, app, id, lang, Some(note)).await
        }
        "collect" => {
            let sums = activities::collect_ready(&app.pool, id).await?;
            show_activities(ctx, c, app, id, lang, Some(embeds::claim_note(&sums, lang))).await
        }
        "city" => show_city(ctx, c, app, id, lang, None).await,
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
                    let p_name = project_name(proj, lang);
                    match lang {
                        Lang::En => format!("Project {p_name} started — finishes <t:{}:R>.", run.ends_at.timestamp()),
                        Lang::Id => format!("Proyek {p_name} dimulai — selesai <t:{}:R>.", run.ends_at.timestamp()),
                    }
                }
                Err(city::CityError::Db(e)) => return Err(e.into()),
                Err(city::CityError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::city_error(&e, lang),
            };
            show_city(ctx, c, app, id, lang, Some(note)).await
        }
        "citydep" => {
            let Ok(bac) = rest.parse::<i64>() else { return Ok(()) };
            let amount = Decimal::new(bac * 100, 2);
            let key = format!("citydep:{}", c.id.get());
            let note = match city::deposit_to_treasury(&app.pool, id, amount, &key).await {
                Ok((wallet, treasury)) => match lang {
                    Lang::En => format!(
                        "Deposited {} into City Treasury. Treasury: {}, Wallet: {}.",
                        format_bac(amount),
                        format_bac(treasury),
                        format_bac(wallet)
                    ),
                    Lang::Id => format!(
                        "Berhasil setor {} ke City Treasury. Treasury: {}, Wallet: {}.",
                        format_bac(amount),
                        format_bac(treasury),
                        format_bac(wallet)
                    ),
                },
                Err(city::CityError::Db(e)) => return Err(e.into()),
                Err(city::CityError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::city_error(&e, lang),
            };
            show_city(ctx, c, app, id, lang, Some(note)).await
        }
        "citytax" => {
            let Ok(pct) = rest.parse::<i32>() else { return Ok(()) };
            let note = match city::set_tax_rate(&app.pool, id, pct).await {
                Ok(new_pct) => match lang {
                    Lang::En => format!("Resident tax rate set to **{}%**.", new_pct),
                    Lang::Id => format!("Tarif pajak warga diatur ke **{}%**.", new_pct),
                },
                Err(city::CityError::Db(e)) => return Err(e.into()),
                Err(city::CityError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::city_error(&e, lang),
            };
            show_city(ctx, c, app, id, lang, Some(note)).await
        }
        "sell" => {
            let v = inventory::view(&app.pool, id).await?;
            if v.lines.is_empty() {
                let (title, body) = match lang {
                    Lang::En => ("Sell to NPC", "Your inventory is empty, nothing to sell yet."),
                    Lang::Id => ("Jual ke NPC", "Inventory kosong, belum ada yang bisa dijual."),
                };
                return update(
                    ctx,
                    c,
                    embeds::notice(title, body),
                    components::main_menu(owner, lang),
                )
                .await;
            }
            let ov = market::overview(&app.pool, id).await?;
            let price_map: HashMap<&'static str, Decimal> =
                ov.prices.iter().map(|p| (p.item.id, p.current_price)).collect();
            let items: Vec<(String, String, i64)> = v
                .lines
                .iter()
                .map(|l| (item_name(l.item.id, lang).to_string(), l.item.id.to_string(), l.quantity))
                .collect();
            update(
                ctx,
                c,
                embeds::sell_prompt(&v, &price_map, lang),
                components::sell_select(owner, &items, lang),
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
                embeds::sell_item(item, owned, unit_price, lang),
                components::sell_qty(owner, item.id, lang),
            )
            .await
        }
        "market" => show_market(ctx, c, app, id, lang, None).await,
        "mktnew" => {
            let v = inventory::view(&app.pool, id).await?;
            if v.lines.is_empty() {
                let msg = match lang {
                    Lang::En => "Inventory is empty, no items to consign to the Broker.",
                    Lang::Id => "Inventory kosong, tidak ada item untuk dititipkan ke Makelar.",
                };
                return show_market(ctx, c, app, id, lang, Some(msg.to_string())).await;
            }
            let items: Vec<(String, String, i64)> = v
                .lines
                .iter()
                .map(|l| (item_name(l.item.id, lang).to_string(), l.item.id.to_string(), l.quantity))
                .collect();
            let (title, body) = match lang {
                Lang::En => ("List with Broker", "Select an item from your inventory to consign."),
                Lang::Id => ("Titip ke Makelar", "Pilih item dari inventory yang ingin dititipkan."),
            };
            update(
                ctx,
                c,
                embeds::notice(title, body),
                components::broker_item_select(owner, &items, lang),
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
            let it_name = item_name(item.id, lang);
            let (title, body) = match lang {
                Lang::En => (
                    format!("Consign {it_name} to Broker"),
                    format!(
                        "You own **{owned}**. Current market price: {}/unit.\nChoose how many to list:",
                        format_bac(cur_price)
                    ),
                ),
                Lang::Id => (
                    format!("Titip {it_name} ke Makelar"),
                    format!(
                        "Kamu punya **{owned}**. Harga pasar saat ini: {}/unit.\nPilih jumlah yang ingin dititipkan:",
                        format_bac(cur_price)
                    ),
                ),
            };
            update(
                ctx,
                c,
                embeds::notice(&title, &body),
                components::broker_qty_select(owner, item.id, owned, lang),
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
            let it_name = item_name(item.id, lang);
            let (title, desc) = match lang {
                Lang::En => (
                    "Choose Target Price & Duration",
                    format!(
                        "Item: **{} × {}**\nCurrent market price: {}/unit\n\n\
                         • **+10% (1 Hr)**: target {}/unit (fee {})\n\
                         • **+25% (6 Hrs)**: target {}/unit (fee {})\n\
                         • **+40% (24 Hrs)**: target {}/unit (fee {})",
                        it_name,
                        qty,
                        format_bac(cur_price),
                        format_bac(p110),
                        format_bac(calc_listing_fee(p110, qty)),
                        format_bac(p125),
                        format_bac(calc_listing_fee(p125, qty)),
                        format_bac(p140),
                        format_bac(calc_listing_fee(p140, qty)),
                    ),
                ),
                Lang::Id => (
                    "Pilih Target Harga & Durasi",
                    format!(
                        "Item: **{} × {}**\nHarga pasar sekarang: {}/unit\n\n\
                         • **+10% (1 Jam)**: target {}/unit (fee {})\n\
                         • **+25% (6 Jam)**: target {}/unit (fee {})\n\
                         • **+40% (24 Jam)**: target {}/unit (fee {})",
                        it_name,
                        qty,
                        format_bac(cur_price),
                        format_bac(p110),
                        format_bac(calc_listing_fee(p110, qty)),
                        format_bac(p125),
                        format_bac(calc_listing_fee(p125, qty)),
                        format_bac(p140),
                        format_bac(calc_listing_fee(p140, qty)),
                    ),
                ),
            };
            update(
                ctx,
                c,
                embeds::notice(title, &desc),
                components::broker_terms_select(owner, item.id, qty, lang),
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
                    let it_name = item_name(l.item.id, lang);
                    match lang {
                        Lang::En => format!(
                            "Listing #{} created: **{} × {}** target {}/unit (fee {}, expires <t:{}:R>).",
                            l.id,
                            it_name,
                            l.quantity,
                            format_bac(l.target_unit_price),
                            format_bac(l.listing_fee),
                            l.expires_at.timestamp()
                        ),
                        Lang::Id => format!(
                            "Listing #{} dibuat: **{} × {}** target {}/unit (fee {}, habis <t:{}:R>).",
                            l.id,
                            it_name,
                            l.quantity,
                            format_bac(l.target_unit_price),
                            format_bac(l.listing_fee),
                            l.expires_at.timestamp()
                        ),
                    }
                }
                Err(market::MarketError::Db(e)) => return Err(e.into()),
                Err(market::MarketError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::market_error(&e, lang),
            };
            show_market(ctx, c, app, id, lang, Some(note)).await
        }
        "craft" => show_craft(ctx, c, app, id, lang, None).await,
        "craftauto" => {
            let on = crafting::toggle_auto_sell(&app.pool, id).await?;
            let note = match (on, lang) {
                (true, Lang::En) => "Auto-Sell for crafted items enabled.",
                (false, Lang::En) => "Auto-Sell for crafted items disabled.",
                (true, Lang::Id) => "Auto-Sell hasil craft diaktifkan.",
                (false, Lang::Id) => "Auto-Sell hasil craft dimatikan.",
            };
            show_craft(ctx, c, app, id, lang, Some(note.to_string())).await
        }
        "craftdo" => {
            let note = match crafting::start_craft(&app.pool, id, rest, app.activity_duration_pct).await {
                Ok(run) => {
                    app.notify.notify_one();
                    let r_name = recipe_name(run.recipe.id, lang);
                    match lang {
                        Lang::En => format!(
                            "Crafting **{} × {}** started — finishes <t:{}:R>.",
                            r_name,
                            run.quantity,
                            run.ends_at.timestamp()
                        ),
                        Lang::Id => format!(
                            "Crafting **{} × {}** dimulai — selesai <t:{}:R>.",
                            r_name,
                            run.quantity,
                            run.ends_at.timestamp()
                        ),
                    }
                }
                Err(crafting::CraftError::Db(e)) => return Err(e.into()),
                Err(crafting::CraftError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::craft_error(&e, lang),
            };
            show_craft(ctx, c, app, id, lang, Some(note)).await
        }
        "orderpick" => {
            let ov = crafting::overview(&app.pool, id).await?;
            if ov.orders.is_empty() {
                let msg = match lang {
                    Lang::En => "No active orders.",
                    Lang::Id => "Tidak ada pesanan aktif.",
                };
                return show_craft(ctx, c, app, id, lang, Some(msg.to_string())).await;
            }
            let items: Vec<(i64, String)> = ov
                .orders
                .iter()
                .map(|o| {
                    (
                        o.id,
                        format!(
                            "#{} {} × {} ({})",
                            o.id,
                            item_name(o.item.id, lang),
                            o.quantity,
                            format_bac(o.reward_bac)
                        ),
                    )
                })
                .collect();
            let (title, body) = match lang {
                Lang::En => ("Fulfill NPC Order", "Select an order to deliver."),
                Lang::Id => ("Penuhi Pesanan NPC", "Pilih pesanan yang ingin diserahkan."),
            };
            update(
                ctx,
                c,
                embeds::notice(title, body),
                components::order_select(owner, &items, lang),
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
                    let it_name = item_name(s.item.id, lang);
                    let lvl = if s.new_rep_level > s.old_rep_level {
                        match lang {
                            Lang::En => format!(" — Reputation rose to Lv {}!", s.new_rep_level),
                            Lang::Id => format!(" — Reputasi naik ke Lv {}!", s.new_rep_level),
                        }
                    } else {
                        String::new()
                    };
                    match lang {
                        Lang::En => format!(
                            "Order #{} (**{} × {}**) fulfilled! Reward {} + {} Rep XP{}.",
                            order_id,
                            it_name,
                            s.quantity,
                            format_bac(s.reward_bac),
                            s.rep_xp_gain,
                            lvl
                        ),
                        Lang::Id => format!(
                            "Pesanan #{} (**{} × {}**) selesai! Hadiah {} + {} Rep XP{}.",
                            order_id,
                            it_name,
                            s.quantity,
                            format_bac(s.reward_bac),
                            s.rep_xp_gain,
                            lvl
                        ),
                    }
                }
                Err(crafting::CraftError::Db(e)) => return Err(e.into()),
                Err(crafting::CraftError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::craft_error(&e, lang),
            };
            show_craft(ctx, c, app, id, lang, Some(note)).await
        }
        "crew" => show_crew(ctx, c, app, id, lang, None).await,
        "crewhire" => {
            let key = format!("crewhire:{}", c.id.get());
            let note = match crew::recruit(&app.pool, id, &key).await {
                Ok(cw) => {
                    let spec = activity_name(cw.specialty, lang);
                    match lang {
                        Lang::En => format!("Hired **{}** [{}] ({spec} Specialist).", cw.name, cw.rarity),
                        Lang::Id => format!("Berhasil merekrut **{}** [{}] (Spesialis {spec}).", cw.name, cw.rarity),
                    }
                }
                Err(crew::CrewError::Db(e)) => return Err(e.into()),
                Err(crew::CrewError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::crew_error(&e, lang),
            };
            show_crew(ctx, c, app, id, lang, Some(note)).await
        }
        "crewupg" => {
            let list = crew::list(&app.pool, id).await?;
            if list.is_empty() {
                let msg = match lang {
                    Lang::En => "No crew members yet.",
                    Lang::Id => "Belum ada kru.",
                };
                return show_crew(ctx, c, app, id, lang, Some(msg.to_string())).await;
            }
            let max_word = match lang {
                Lang::En => "Max",
                Lang::Id => "Maks",
            };
            let items: Vec<(i64, String)> = list
                .iter()
                .map(|cw| {
                    let cost = cw.upgrade_cost.map(format_bac).unwrap_or_else(|| max_word.to_string());
                    (cw.id, format!("#{} {} Lv {} ({})", cw.id, cw.name, cw.level, cost))
                })
                .collect();
            let (title, body, ph) = match lang {
                Lang::En => ("Upgrade Crew", "Select a crew member to upgrade.", "Select crew"),
                Lang::Id => ("Upgrade Kru", "Pilih kru yang ingin di-upgrade.", "Pilih kru"),
            };
            update(
                ctx,
                c,
                embeds::notice(title, body),
                components::crew_select(owner, "crewupgpick", &items, ph, lang),
            )
            .await
        }
        "crewupgpick" => {
            let picked = match &c.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => values.first().cloned(),
                _ => None,
            };
            let Some(crew_id) = picked.and_then(|s| s.parse::<i64>().ok()) else { return Ok(()) };
            let key = format!("crewupg:{}", c.id.get());
            let note = match crew::upgrade(&app.pool, id, crew_id, &key).await {
                Ok(cw) => match lang {
                    Lang::En => format!("**{}** upgraded to Lv {}!", cw.name, cw.level),
                    Lang::Id => format!("**{}** naik ke Lv {}!", cw.name, cw.level),
                },
                Err(crew::CrewError::Db(e)) => return Err(e.into()),
                Err(crew::CrewError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::crew_error(&e, lang),
            };
            show_crew(ctx, c, app, id, lang, Some(note)).await
        }
        "crewassign" => {
            let list = crew::list(&app.pool, id).await?;
            if list.is_empty() {
                let msg = match lang {
                    Lang::En => "No crew members yet.",
                    Lang::Id => "Belum ada kru.",
                };
                return show_crew(ctx, c, app, id, lang, Some(msg.to_string())).await;
            }
            let items: Vec<(i64, String)> = list
                .iter()
                .map(|cw| (cw.id, format!("#{} {} [{} Lv {}]", cw.id, cw.name, cw.rarity, cw.level)))
                .collect();
            let (title, body, ph) = match lang {
                Lang::En => ("Assign Crew", "Select a crew member to assign to an activity.", "Select crew"),
                Lang::Id => ("Tugaskan Kru", "Pilih kru untuk ditugaskan ke aktivitas.", "Pilih kru"),
            };
            update(
                ctx,
                c,
                embeds::notice(title, body),
                components::crew_select(owner, "crewassignpick", &items, ph, lang),
            )
            .await
        }
        "crewassignpick" => {
            let picked = match &c.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => values.first().cloned(),
                _ => None,
            };
            let Some(crew_id) = picked.and_then(|s| s.parse::<i64>().ok()) else { return Ok(()) };
            let (title, body) = match lang {
                Lang::En => ("Choose Activity Assignment", "Select an activity and automation mode for this crew member."),
                Lang::Id => ("Pilih Tugas Aktivitas", "Pilih aktivitas dan mode otomasi untuk kru ini."),
            };
            update(
                ctx,
                c,
                embeds::notice(title, body),
                components::crew_assign_activity(owner, crew_id, lang),
            )
            .await
        }
        "crewset" => {
            let parts: Vec<&str> = rest.splitn(3, ':').collect();
            if parts.len() != 3 {
                return Ok(());
            }
            let Ok(crew_id) = parts[0].parse::<i64>() else { return Ok(()) };
            let act = if parts[1] == "none" { None } else { ActivityKind::parse(parts[1]) };
            let auto = parts[2] == "1";
            let note = match crew::assign(&app.pool, id, crew_id, act, auto).await {
                Ok(cw) => match cw.assigned {
                    Some(a) if cw.auto_enabled => {
                        let a_name = activity_name(a, lang);
                        match lang {
                            Lang::En => format!("**{}** assigned to {a_name} (AUTO).", cw.name),
                            Lang::Id => format!("**{}** ditugaskan ke {a_name} (AUTO).", cw.name),
                        }
                    }
                    Some(a) => {
                        let a_name = activity_name(a, lang);
                        match lang {
                            Lang::En => format!("**{}** assigned to {a_name} (Manual).", cw.name),
                            Lang::Id => format!("**{}** ditugaskan ke {a_name} (Manual).", cw.name),
                        }
                    }
                    None => match lang {
                        Lang::En => format!("**{}** unassigned from task.", cw.name),
                        Lang::Id => format!("**{}** dilepas dari tugas.", cw.name),
                    },
                },
                Err(crew::CrewError::Db(e)) => return Err(e.into()),
                Err(crew::CrewError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::crew_error(&e, lang),
            };
            show_crew(ctx, c, app, id, lang, Some(note)).await
        }
        "mktcancel" => {
            let ov = market::overview(&app.pool, id).await?;
            if ov.listings.is_empty() {
                let msg = match lang {
                    Lang::En => "No active listings.",
                    Lang::Id => "Tidak ada listing aktif.",
                };
                return show_market(ctx, c, app, id, lang, Some(msg.to_string())).await;
            }
            let items: Vec<(i64, String)> = ov
                .listings
                .iter()
                .map(|l| {
                    (
                        l.id,
                        format!(
                            "#{} {} × {} ({}/unit)",
                            l.id,
                            item_name(l.item.id, lang),
                            l.quantity,
                            format_bac(l.target_unit_price)
                        ),
                    )
                })
                .collect();
            let (title, body) = match lang {
                Lang::En => ("Cancel Listing", "Select a listing to cancel (listing fee is non-refundable)."),
                Lang::Id => ("Batalkan Listing", "Pilih listing yang ingin dibatalkan (fee tidak dikembalikan)."),
            };
            update(
                ctx,
                c,
                embeds::notice(title, body),
                components::broker_cancel_select(owner, &items, lang),
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
                Ok((item, stored, pending)) if pending > 0 => {
                    let it_name = item_name(item.id, lang);
                    match lang {
                        Lang::En => format!(
                            "Listing #{} cancelled. {} +{} returned to inventory, {} moved to Pending Claim.",
                            listing_id, it_name, stored, pending
                        ),
                        Lang::Id => format!(
                            "Listing #{} dibatalkan. {} +{} kembali ke inventory, {} masuk Pending Claim.",
                            listing_id, it_name, stored, pending
                        ),
                    }
                }
                Ok((item, stored, _)) => {
                    let it_name = item_name(item.id, lang);
                    match lang {
                        Lang::En => format!(
                            "Listing #{} cancelled. {} +{} returned to inventory.",
                            listing_id, it_name, stored
                        ),
                        Lang::Id => format!(
                            "Listing #{} dibatalkan. {} +{} dikembalikan ke inventory.",
                            listing_id, it_name, stored
                        ),
                    }
                }
                Err(market::MarketError::Db(e)) => return Err(e.into()),
                Err(market::MarketError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::market_error(&e, lang),
            };
            show_market(ctx, c, app, id, lang, Some(note)).await
        }
        "sellcustom" => {
            let Some(item) = find_item(rest) else { return Ok(()) };
            let owned = inventory::view(&app.pool, id)
                .await?
                .lines
                .iter()
                .find(|l| l.item.id == item.id)
                .map(|l| l.quantity)
                .unwrap_or(0);
            let modal = components::sell_custom_modal(
                owner,
                item.id,
                item_name(item.id, lang),
                owned,
                lang,
            );
            c.create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
                .await?;
            Ok(())
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
                Ok(SaleOutcome::Sold { item, qty, total, balance }) => {
                    let it_name = item_name(item.id, lang);
                    match lang {
                        Lang::En => format!(
                            "Sold {} {} for {}. Balance: {}.",
                            qty,
                            it_name,
                            format_bac(total),
                            format_bac(balance)
                        ),
                        Lang::Id => format!(
                            "Terjual {} {} seharga {}. Saldo: {}.",
                            qty,
                            it_name,
                            format_bac(total),
                            format_bac(balance)
                        ),
                    }
                }
                Ok(SaleOutcome::Duplicate) => match lang {
                    Lang::En => "This sale has already been processed.".to_string(),
                    Lang::Id => "Penjualan ini sudah diproses.".to_string(),
                },
                Err(SellError::Db(e)) => return Err(e.into()),
                Err(SellError::Wallet(e)) => return Err(e.into()),
                Err(e) => {
                    let err_txt = i18n_errors::sell_error(&e, lang);
                    match lang {
                        Lang::En => format!("Sale failed: {err_txt}"),
                        Lang::Id => format!("Gagal menjual: {err_txt}"),
                    }
                }
            };
            let title = match lang {
                Lang::En => "Sell to NPC",
                Lang::Id => "Jual ke NPC",
            };
            update(
                ctx,
                c,
                embeds::notice(title, &text),
                components::main_menu(owner, lang),
            )
            .await
        }
        _ => Ok(()),
    }
}

fn modal_field<'a>(m: &'a ModalInteraction, custom_id: &str) -> &'a str {
    m.data
        .components
        .iter()
        .flat_map(|r| r.components.iter())
        .find_map(|c| match c {
            ActionRowComponent::InputText(t) if t.custom_id == custom_id => t.value.as_deref(),
            _ => None,
        })
        .unwrap_or("")
        .trim()
}

async fn reply_modal_ephemeral(
    ctx: &Context,
    m: &ModalInteraction,
    text: &str,
) -> anyhow::Result<()> {
    let msg = CreateInteractionResponseMessage::new()
        .content(text)
        .ephemeral(true);
    m.create_response(&ctx.http, CreateInteractionResponse::Message(msg))
        .await?;
    Ok(())
}

pub async fn handle_modal(
    ctx: &Context,
    m: &ModalInteraction,
    app: &AppState,
) -> anyhow::Result<()> {
    let Some((action, owner, rest)) = components::parse_custom_id(&m.data.custom_id) else {
        return Ok(());
    };
    if m.user.id.get() != owner {
        return reply_modal_ephemeral(ctx, m, "This modal belongs to another player.").await;
    }

    match action {
        "sellmodal" => {
            let item_id = rest;
            let raw_qty = modal_field(m, "qty");
            let lang = player::get_lang(&app.pool, owner as i64)
                .await
                .unwrap_or(Lang::En);

            let text = match raw_qty.parse::<i64>() {
                Ok(n) if n > 0 => {
                    let key = format!("sell:{}", m.id.get());
                    match economy::sell_to_npc(&app.pool, owner as i64, item_id, SellQty::Exact(n), &key).await {
                        Ok(SaleOutcome::Sold { item, qty, total, balance }) => {
                            let it_name = item_name(item.id, lang);
                            match lang {
                                Lang::En => format!(
                                    "Sold {} {} for {}. Balance: {}.",
                                    qty,
                                    it_name,
                                    format_bac(total),
                                    format_bac(balance)
                                ),
                                Lang::Id => format!(
                                    "Terjual {} {} seharga {}. Saldo: {}.",
                                    qty,
                                    it_name,
                                    format_bac(total),
                                    format_bac(balance)
                                ),
                            }
                        }
                        Ok(SaleOutcome::Duplicate) => match lang {
                            Lang::En => "This sale has already been processed.".to_string(),
                            Lang::Id => "Penjualan ini sudah diproses.".to_string(),
                        },
                        Err(SellError::Db(e)) => return Err(e.into()),
                        Err(SellError::Wallet(e)) => return Err(e.into()),
                        Err(e) => {
                            let err_txt = i18n_errors::sell_error(&e, lang);
                            match lang {
                                Lang::En => format!("Sale failed: {err_txt}"),
                                Lang::Id => format!("Gagal menjual: {err_txt}"),
                            }
                        }
                    }
                }
                _ => match lang {
                    Lang::En => "Invalid quantity. Enter a whole number greater than 0.".to_string(),
                    Lang::Id => "Jumlah tidak valid. Masukkan angka bulat lebih dari 0.".to_string(),
                },
            };

            let title = match lang {
                Lang::En => "Sell to NPC",
                Lang::Id => "Jual ke NPC",
            };
            let msg = CreateInteractionResponseMessage::new()
                .embed(embeds::notice(title, &text))
                .components(components::main_menu(owner, lang));
            m.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg))
                .await?;
            Ok(())
        }
        "welcomemodal" => {
            let Some(guild_id) = m.guild_id else {
                return reply_modal_ephemeral(ctx, m, "This modal can only be used inside a server.").await;
            };
            let mut parts = rest.splitn(2, ':');
            let (Some(ch_s), Some(role_s)) = (parts.next(), parts.next()) else {
                return Ok(());
            };
            let (Ok(channel_id), Ok(role_id)) = (ch_s.parse::<u64>(), role_s.parse::<u64>()) else {
                return Ok(());
            };

            let title = modal_field(m, "title").to_string();
            let message = modal_field(m, "message").to_string();
            if title.is_empty() || message.is_empty() {
                return reply_modal_ephemeral(ctx, m, "Title and message cannot be empty.").await;
            }

            let cfg = WelcomeConfig {
                guild_id: guild_id.get() as i64,
                channel_id: channel_id as i64,
                auto_role_id: (role_id != 0).then_some(role_id as i64),
                title,
                message,
                enabled: true,
            };
            server::save_welcome(&app.pool, &app.welcome_cache, cfg).await?;
            let role_note = if role_id != 0 {
                format!("\nAuto-Role: <@&{role_id}>")
            } else {
                "\nAuto-Role: None".to_string()
            };
            reply_modal_ephemeral(
                ctx,
                m,
                &format!(
                    "✅ Welcome greet saved for <#{channel_id}>.{role_note}\nRun `/welcome test` to preview it."
                ),
            )
            .await
        }
        "announcemodal" => {
            let mut parts = rest.splitn(3, ':');
            let (Some(ch_s), Some(role_s), Some(ev_s)) = (parts.next(), parts.next(), parts.next()) else {
                return Ok(());
            };
            let (Ok(channel_id), Ok(role_id)) = (ch_s.parse::<u64>(), role_s.parse::<u64>()) else {
                return Ok(());
            };
            let ping_everyone = ev_s == "1";

            let title = modal_field(m, "title");
            let body = modal_field(m, "body");
            let raw_img = modal_field(m, "image_url");
            if title.is_empty() || body.is_empty() {
                return reply_modal_ephemeral(ctx, m, "Title and body cannot be empty.").await;
            }
            let image_url = if raw_img.is_empty() {
                None
            } else if raw_img.starts_with("http://") || raw_img.starts_with("https://") {
                Some(raw_img)
            } else {
                return reply_modal_ephemeral(
                    ctx,
                    m,
                    "Image URL must start with `http://` or `https://`.",
                )
                .await;
            };

            let mut mentions = Vec::new();
            if ping_everyone {
                mentions.push("@everyone".to_string());
            }
            if role_id != 0 {
                mentions.push(format!("<@&{role_id}>"));
            }
            let content = mentions.join(" ");

            let mut allowed = CreateAllowedMentions::new().everyone(ping_everyone);
            if role_id != 0 {
                allowed = allowed.roles([RoleId::new(role_id)]);
            }

            let author_tag = m
                .user
                .global_name
                .as_deref()
                .unwrap_or(m.user.name.as_str());
            let mut msg = CreateMessage::new()
                .embed(server_ui::announcement_embed(title, body, image_url, author_tag))
                .allowed_mentions(allowed);
            if !content.is_empty() {
                msg = msg.content(content);
            }

            m.defer_ephemeral(&ctx.http).await?;
            let reply = match ChannelId::new(channel_id).send_message(&ctx.http, msg).await {
                Ok(_) => format!("📢 Announcement posted to <#{channel_id}>."),
                Err(e) => format!("Failed to post announcement to <#{channel_id}>: {e}"),
            };
            m.edit_response(&ctx.http, EditInteractionResponse::new().content(reply))
                .await?;
            Ok(())
        }
        _ => Ok(()),
    }
}
