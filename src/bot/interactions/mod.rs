use crate::application::economy::{self, SaleOutcome, SellError, SellQty};
use crate::application::giveaway::{self, EnterOutcome};
use crate::application::{activities, city, crafting, crew, inventory, market, player};
use crate::bot::{commands, components, embeds, giveaway_ui, i18n_errors, AppState};
use crate::domain::activities::ActivityKind;
use crate::domain::city::ProjectId;
use crate::domain::crafting::{find_recipe, max_craftable_batches};
use crate::domain::economy::format_bac;
use crate::domain::i18n::{
    activity_name, item_name, project_name, recipe_name, Lang,
};
use crate::domain::market::{calc_listing_fee, calc_sale_settlement, ListingDuration};
use crate::domain::resources::find_item;
use rust_decimal::Decimal;
use serenity::all::*;
use std::collections::HashMap;

pub async fn handle_command(
    ctx: &Context,
    cmd: &CommandInteraction,
    app: &AppState,
) -> anyhow::Result<()> {
    if cmd.data.kind == CommandType::PrimaryEntryPoint {
        cmd.create_response(&ctx.http, CreateInteractionResponse::LaunchActivity)
            .await?;
        return Ok(());
    }
    match cmd.data.name.as_str() {
        "help" => commands::help::run(ctx, cmd, app).await,
        "setup" => commands::setup::run(ctx, cmd, app).await,
        "giveaway" => commands::giveaway::run(ctx, cmd, app).await,
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
        components::activity_menu(id as u64, ov.stage, lang),
    )
    .await
}

async fn build_city_view(
    pool: &sqlx::PgPool,
    id: i64,
    lang: Lang,
    note: Option<String>,
) -> anyhow::Result<(CreateEmbed, Vec<CreateActionRow>)> {
    let ov = city::overview(pool, id).await?;
    let built: Vec<(ProjectId, i32)> = ov.facilities.iter().map(|f| (f.id, f.level)).collect();
    Ok((
        embeds::city(&ov, note, lang),
        components::city_menu(id as u64, ov.stage, &built, lang),
    ))
}

async fn show_city(
    ctx: &Context,
    c: &ComponentInteraction,
    app: &AppState,
    id: i64,
    lang: Lang,
    note: Option<String>,
) -> anyhow::Result<()> {
    let (embed, rows) = build_city_view(&app.pool, id, lang, note).await?;
    update(ctx, c, embed, rows).await
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

async fn build_craft_view(
    pool: &sqlx::PgPool,
    id: i64,
    lang: Lang,
    note: Option<String>,
) -> anyhow::Result<(CreateEmbed, Vec<CreateActionRow>)> {
    let ov = crafting::overview(pool, id).await?;
    let auto_sell = ov.auto_sell;
    Ok((
        embeds::craft(&ov, note, lang),
        components::craft_menu(id as u64, auto_sell, lang),
    ))
}

async fn show_craft(
    ctx: &Context,
    c: &ComponentInteraction,
    app: &AppState,
    id: i64,
    lang: Lang,
    note: Option<String>,
) -> anyhow::Result<()> {
    let (embed, rows) = build_craft_view(&app.pool, id, lang, note).await?;
    update(ctx, c, embed, rows).await
}

async fn player_stock(pool: &sqlx::PgPool, id: i64) -> anyhow::Result<HashMap<&'static str, i64>> {
    let v = inventory::view(pool, id).await?;
    Ok(v.lines.into_iter().map(|l| (l.item.id, l.quantity)).collect())
}

async fn run_craft_batch_note(
    app: &AppState,
    id: i64,
    recipe_id: &str,
    batch_qty: i64,
    lang: Lang,
) -> anyhow::Result<String> {
    let voice_spd = app.voice_boost.speed_bonus_pct(id);
    match crafting::start_craft_batch_with_bonus(
        &app.pool,
        id,
        recipe_id,
        batch_qty,
        app.activity_duration_pct,
        voice_spd,
    )
    .await
    {
        Ok(run) => {
            app.notify.notify_one();
            let r_name = recipe_name(run.recipe.id, lang);
            Ok(format!(
                "Crafting **{} × {}** started — finishes <t:{}:R>.",
                r_name,
                run.quantity,
                run.ends_at.timestamp()
            ))
        }
        Err(crafting::CraftError::Db(e)) => Err(e.into()),
        Err(crafting::CraftError::Wallet(e)) => Err(e.into()),
        Err(e) => Ok(i18n_errors::craft_error(&e, lang)),
    }
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
        .embed(embeds::with_voice_boost(
            embeds::welcome(&profile, created, lang),
            app.voice_boost.is_boosted(pid),
        ))
        .components(components::profile_menu(uid.get(), lang));

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
    if action == "launch" && owner == 0 {
        c.create_response(&ctx.http, CreateInteractionResponse::LaunchActivity)
            .await?;
        return Ok(());
    }
    if action == "gwjoin" && owner == 0 {
        let Ok(giveaway_id) = rest.parse::<i64>() else {
            return Ok(());
        };
        return match giveaway::enter_giveaway(&app.pool, giveaway_id, c.user.id.get() as i64).await? {
            EnterOutcome::Entered { giveaway, total_entries } => {
                let msg = CreateInteractionResponseMessage::new()
                    .embed(giveaway_ui::giveaway_active_embed(&giveaway, total_entries))
                    .components(giveaway_ui::giveaway_join_button(giveaway.id, total_entries, false));
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
                    "You need an Urbaniac profile before joining this giveaway! Click **Play** or open the **Urbaniac Activity**, then try again.",
                )
                .await
            }
            EnterOutcome::Ended => {
                deny(ctx, c, "This giveaway has already ended.").await
            }
            EnterOutcome::NotFound => {
                deny(ctx, c, "Giveaway not found.").await
            }
        };
    }
    // Validasi pemilik data.
    if c.user.id.get() != owner {
        return deny(
            ctx,
            c,
            "This menu belongs to another player. Click **Play** or open the **Urbaniac Activity** for your own menu.",
        )
        .await;
    }
    let id = owner as i64;
    let Some(pl) = player::get_player(&app.pool, id).await? else {
        return deny(ctx, c, "No profile yet. Click **Play** or open the **Urbaniac Activity**.").await;
    };
    let lang = pl.language();

    match action {
        "lang" | "profile" => {
            let Some(profile) = player::get_player_profile(&app.pool, id).await? else {
                return deny(ctx, c, "No profile yet. Click **Play** or open the **Urbaniac Activity**.").await;
            };
            update(
                ctx,
                c,
                embeds::with_voice_boost(
                    embeds::profile(&profile, lang),
                    app.voice_boost.is_boosted(id),
                ),
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
            let has_items = !v.lines.is_empty();
            let has_pending = !v.pending.is_empty();
            update(
                ctx,
                c,
                embeds::inventory(&v, None, lang),
                components::inventory_menu(owner, has_items, has_pending, lang),
            )
            .await
        }
        "claim" => {
            let results = inventory::claim_pending(&app.pool, id).await?;
            let note = if results.is_empty() {
                "No Pending Claims.".to_string()
            } else {
                results
                    .iter()
                    .map(|r| {
                        let name = item_name(&r.item_id, lang);
                        if r.remaining == 0 {
                            format!("{name}: {} claimed", r.claimed)
                        } else {
                            format!(
                                "{name}: {} claimed, {} still pending (inventory full)",
                                r.claimed, r.remaining
                            )
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let v = inventory::view(&app.pool, id).await?;
            let has_items = !v.lines.is_empty();
            let has_pending = !v.pending.is_empty();
            update(
                ctx,
                c,
                embeds::inventory(&v, Some(note), lang),
                components::inventory_menu(owner, has_items, has_pending, lang),
            )
            .await
        }
        "activities" => show_activities(ctx, c, app, id, lang, None).await,
        "startact" => {
            let Some(kind) = ActivityKind::parse(rest) else { return Ok(()) };
            let voice_spd = app.voice_boost.speed_bonus_pct(id);
            let note = match activities::start_with_bonus(
                &app.pool,
                id,
                kind,
                app.activity_duration_pct,
                voice_spd,
            )
            .await
            {
                Ok(run) => {
                    app.notify.notify_one(); // bangunkan scheduler
                    let a_name = activity_name(kind, lang);
                    format!("{a_name} started — finishes <t:{}:R>.", run.ends_at.timestamp())
                }
                Err(activities::ActivityError::Db(e)) => return Err(e.into()),
                Err(e) => i18n_errors::activity_error(&e, lang),
            };
            show_activities(ctx, c, app, id, lang, Some(note)).await
        }
        "collect" => {
            let xp_bonus = app.voice_boost.xp_bonus_pct(id);
            let sums = activities::collect_ready_with_xp_bonus(&app.pool, id, xp_bonus).await?;
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
                    format!("Project {p_name} started — finishes <t:{}:R>.", run.ends_at.timestamp())
                }
                Err(city::CityError::Db(e)) => return Err(e.into()),
                Err(city::CityError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::city_error(&e, lang),
            };
            show_city(ctx, c, app, id, lang, Some(note)).await
        }
        "citydepcustom" => {
            let bal = economy::balance(&app.pool, id).await?;
            let modal = components::city_deposit_modal(owner, bal, lang);
            c.create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
                .await?;
            Ok(())
        }
        "citywdcustom" => {
            let ov = city::overview(&app.pool, id).await?;
            let modal = components::city_withdraw_modal(owner, ov.treasury, lang);
            c.create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
                .await?;
            Ok(())
        }
        "citydep" => {
            let Ok(bac) = rest.parse::<i64>() else { return Ok(()) };
            let amount = Decimal::new(bac * 100, 2);
            let key = format!("citydep:{}", c.id.get());
            let note = match city::deposit_to_treasury(&app.pool, id, amount, &key).await {
                Ok((wallet, treasury)) => format!(
                    "Deposited {} into City Treasury. Treasury: {}, Wallet: {}.",
                    format_bac(amount),
                    format_bac(treasury),
                    format_bac(wallet)
                ),
                Err(city::CityError::Db(e)) => return Err(e.into()),
                Err(city::CityError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::city_error(&e, lang),
            };
            show_city(ctx, c, app, id, lang, Some(note)).await
        }
        "citytax" => {
            let Ok(pct) = rest.parse::<i32>() else { return Ok(()) };
            let note = match city::set_tax_rate(&app.pool, id, pct).await {
                Ok(new_pct) => format!("Resident tax rate set to **{}%**.", new_pct),
                Err(city::CityError::Db(e)) => return Err(e.into()),
                Err(city::CityError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::city_error(&e, lang),
            };
            show_city(ctx, c, app, id, lang, Some(note)).await
        }
        "sell" => {
            let v = inventory::view(&app.pool, id).await?;
            if v.lines.is_empty() {
                return update(
                    ctx,
                    c,
                    embeds::notice("Sell to NPC", "Your inventory is empty, nothing to sell yet."),
                    components::main_menu(owner, lang),
                )
                .await;
            }
            let ov = market::overview(&app.pool, id).await?;
            let city_ov = city::overview(&app.pool, id).await?;
            let has_mh = city_ov
                .facilities
                .iter()
                .any(|f| f.id == ProjectId::MarketHall && f.level > 0);
            let price_map: HashMap<&'static str, Decimal> = ov
                .prices
                .iter()
                .map(|p| {
                    let price = if has_mh {
                        (p.current_price * Decimal::new(105, 2)).round_dp(2)
                    } else {
                        p.current_price
                    };
                    (p.item.id, price)
                })
                .collect();
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
            if owned <= 0 {
                return update(
                    ctx,
                    c,
                    embeds::notice("Sell to NPC", "You no longer have this item in your inventory."),
                    components::main_menu(owner, lang),
                )
                .await;
            }
            let unit_price = economy::sell_unit_price(&app.pool, id, item).await?;
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
                return show_market(
                    ctx,
                    c,
                    app,
                    id,
                    lang,
                    Some("Inventory is empty, no items to consign to the Broker.".to_string()),
                )
                .await;
            }
            let items: Vec<(String, String, i64)> = v
                .lines
                .iter()
                .map(|l| (item_name(l.item.id, lang).to_string(), l.item.id.to_string(), l.quantity))
                .collect();
            update(
                ctx,
                c,
                embeds::notice("List with Broker", "Select an item from your inventory to consign."),
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
            if owned <= 0 {
                return show_market(
                    ctx,
                    c,
                    app,
                    id,
                    lang,
                    Some("You no longer have this item in your inventory.".to_string()),
                )
                .await;
            }
            let cur_price = market::current_npc_price(&app.pool, item).await?;
            let it_name = item_name(item.id, lang);
            let title = format!("Consign {it_name} to Broker");
            let body = format!(
                "You own **{owned}**. Current market price: {}/unit.\nChoose how many to list:",
                format_bac(cur_price)
            );
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
            let p120 = (cur_price * Decimal::new(120, 2)).round_dp(2);
            let p135 = (cur_price * Decimal::new(135, 2)).round_dp(2);
            let (_, _, net110) = calc_sale_settlement(p110, qty);
            let (_, _, net120) = calc_sale_settlement(p120, qty);
            let (_, _, net135) = calc_sale_settlement(p135, qty);
            let it_name = item_name(item.id, lang);
            let desc = format!(
                "Item: **{} × {}**\nCurrent market price: {}/unit\n\n\
                 • **+10% (2 Hrs)**: target {}/unit (fee {}, net ~{})\n\
                 • **+20% (6 Hrs)**: target {}/unit (fee {}, net ~{})\n\
                 • **+35% (24 Hrs)**: target {}/unit (fee {}, net ~{})",
                it_name,
                qty,
                format_bac(cur_price),
                format_bac(p110),
                format_bac(calc_listing_fee(p110, qty)),
                format_bac(net110),
                format_bac(p120),
                format_bac(calc_listing_fee(p120, qty)),
                format_bac(net120),
                format_bac(p135),
                format_bac(calc_listing_fee(p135, qty)),
                format_bac(net135),
            );
            update(
                ctx,
                c,
                embeds::notice("Choose Target Price & Duration", &desc),
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
                    format!(
                        "Listing #{} created: **{} × {}** target {}/unit (fee {}, expires <t:{}:R>).",
                        l.id,
                        it_name,
                        l.quantity,
                        format_bac(l.target_unit_price),
                        format_bac(l.listing_fee),
                        l.expires_at.timestamp()
                    )
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
            let note = if on {
                "Auto-Sell for crafted items enabled."
            } else {
                "Auto-Sell for crafted items disabled."
            };
            show_craft(ctx, c, app, id, lang, Some(note.to_string())).await
        }
        "craftpick" => {
            let Some(recipe) = find_recipe(rest) else { return Ok(()) };
            let stock = player_stock(&app.pool, id).await?;
            let max_batch = max_craftable_batches(recipe, &stock);
            update(
                ctx,
                c,
                embeds::craft_item(recipe, &stock, max_batch, lang),
                components::craft_qty(owner, recipe.id, max_batch, lang),
            )
            .await
        }
        "craftbatch" => {
            let Some((recipe_id, qty_s)) = rest.split_once(':') else { return Ok(()) };
            let Some(recipe) = find_recipe(recipe_id) else { return Ok(()) };
            let batch_qty = if qty_s == "max" {
                let stock = player_stock(&app.pool, id).await?;
                max_craftable_batches(recipe, &stock).max(1)
            } else {
                let Ok(q) = qty_s.parse::<i64>() else { return Ok(()) };
                q
            };
            let note = run_craft_batch_note(app, id, recipe.id, batch_qty, lang).await?;
            show_craft(ctx, c, app, id, lang, Some(note)).await
        }
        "craftcustom" => {
            let Some(recipe) = find_recipe(rest) else { return Ok(()) };
            let stock = player_stock(&app.pool, id).await?;
            let max_batch = max_craftable_batches(recipe, &stock);
            let modal = components::craft_custom_modal(
                owner,
                recipe.id,
                recipe_name(recipe.id, lang),
                max_batch,
                lang,
            );
            c.create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
                .await?;
            Ok(())
        }
        "craftdo" => {
            let note = run_craft_batch_note(app, id, rest, 1, lang).await?;
            show_craft(ctx, c, app, id, lang, Some(note)).await
        }
        "orderpick" => {
            let ov = crafting::overview(&app.pool, id).await?;
            if ov.orders.is_empty() {
                return show_craft(ctx, c, app, id, lang, Some("No active orders.".to_string())).await;
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
            update(
                ctx,
                c,
                embeds::notice("Fulfill NPC Order", "Select an order to deliver."),
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
            let xp_bonus = app.voice_boost.xp_bonus_pct(id);
            let note = match crafting::fulfill_order_with_xp_bonus(&app.pool, id, order_id, xp_bonus).await {
                Ok(s) => {
                    let it_name = item_name(s.item.id, lang);
                    let lvl = if s.new_rep_level > s.old_rep_level {
                        format!(" — Reputation rose to Lv {}!", s.new_rep_level)
                    } else {
                        String::new()
                    };
                    format!(
                        "Order #{} (**{} × {}**) fulfilled! Reward {} + {} Rep XP{}.",
                        order_id,
                        it_name,
                        s.quantity,
                        format_bac(s.reward_bac),
                        s.rep_xp_gain,
                        lvl
                    )
                }
                Err(crafting::CraftError::Db(e)) => return Err(e.into()),
                Err(crafting::CraftError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::craft_error(&e, lang),
            };
            show_craft(ctx, c, app, id, lang, Some(note)).await
        }
        "orderrefresh" => {
            let note = match crafting::refresh_orders(&app.pool, id).await {
                Ok(()) => "Loaded 3 new NPC orders!".to_string(),
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
                    format!("Hired **{}** [{}] ({spec} Specialist).", cw.name, cw.rarity)
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
                return show_crew(ctx, c, app, id, lang, Some("No crew members yet.".to_string())).await;
            }
            let items: Vec<(i64, String)> = list
                .iter()
                .map(|cw| {
                    let cost = cw.upgrade_cost.map(format_bac).unwrap_or_else(|| "Max".to_string());
                    (cw.id, format!("#{} {} Lv {} ({})", cw.id, cw.name, cw.level, cost))
                })
                .collect();
            update(
                ctx,
                c,
                embeds::notice("Upgrade Crew", "Select a crew member to upgrade."),
                components::crew_select(owner, "crewupgpick", &items, "Select crew", lang),
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
                Ok(cw) => format!("**{}** upgraded to Lv {}!", cw.name, cw.level),
                Err(crew::CrewError::Db(e)) => return Err(e.into()),
                Err(crew::CrewError::Wallet(e)) => return Err(e.into()),
                Err(e) => i18n_errors::crew_error(&e, lang),
            };
            show_crew(ctx, c, app, id, lang, Some(note)).await
        }
        "crewassign" => {
            let list = crew::list(&app.pool, id).await?;
            if list.is_empty() {
                return show_crew(ctx, c, app, id, lang, Some("No crew members yet.".to_string())).await;
            }
            let items: Vec<(i64, String)> = list
                .iter()
                .map(|cw| (cw.id, format!("#{} {} [{} Lv {}]", cw.id, cw.name, cw.rarity, cw.level)))
                .collect();
            update(
                ctx,
                c,
                embeds::notice("Assign Crew", "Select a crew member to assign to an activity."),
                components::crew_select(owner, "crewassignpick", &items, "Select crew", lang),
            )
            .await
        }
        "crewassignpick" => {
            let picked = match &c.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => values.first().cloned(),
                _ => None,
            };
            let Some(crew_id) = picked.and_then(|s| s.parse::<i64>().ok()) else { return Ok(()) };
            let stage = city::stage_for(&app.pool, id).await?;
            update(
                ctx,
                c,
                embeds::notice(
                    "Choose Activity Assignment",
                    "Select an activity and automation mode for this crew member.",
                ),
                components::crew_assign_activity(owner, crew_id, stage, lang),
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
                        let mut base = format!("**{}** assigned to {a_name} (AUTO).", cw.name);
                        if !activities::has_active_kind(&app.pool, id, a).await? {
                            let voice_spd = app.voice_boost.speed_bonus_pct(id);
                            match activities::start_with_bonus(
                                &app.pool,
                                id,
                                a,
                                app.activity_duration_pct,
                                voice_spd,
                            )
                            .await
                            {
                                Ok(run) => {
                                    app.notify.notify_one();
                                    base.push_str(&format!(
                                        " Started {a_name} automatically — finishes <t:{}:R>.",
                                        run.ends_at.timestamp()
                                    ));
                                }
                                Err(activities::ActivityError::NoFreeSlot(_)) => {
                                    base.push_str(
                                        " All Activity Slots are currently busy — AUTO will loop once a slot runs it.",
                                    );
                                }
                                Err(_) => {}
                            }
                        }
                        base
                    }
                    Some(a) => {
                        let a_name = activity_name(a, lang);
                        format!("**{}** assigned to {a_name} (Manual).", cw.name)
                    }
                    None => format!("**{}** unassigned from task.", cw.name),
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
                return show_market(ctx, c, app, id, lang, Some("No active listings.".to_string())).await;
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
            update(
                ctx,
                c,
                embeds::notice(
                    "Cancel Listing",
                    "Select a listing to cancel (listing fee is non-refundable).",
                ),
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
                    format!(
                        "Listing #{} cancelled. {} +{} returned to inventory, {} moved to Pending Claim.",
                        listing_id, it_name, stored, pending
                    )
                }
                Ok((item, stored, _)) => {
                    let it_name = item_name(item.id, lang);
                    format!(
                        "Listing #{} cancelled. {} +{} returned to inventory.",
                        listing_id, it_name, stored
                    )
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
            if owned <= 0 {
                return update(
                    ctx,
                    c,
                    embeds::notice("Sell to NPC", "You no longer have this item in your inventory."),
                    components::main_menu(owner, lang),
                )
                .await;
            }
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
                    format!(
                        "Sold {} {} for {}. Balance: {}.",
                        qty,
                        it_name,
                        format_bac(total),
                        format_bac(balance)
                    )
                }
                Ok(SaleOutcome::Duplicate) => "This sale has already been processed.".to_string(),
                Err(SellError::Db(e)) => return Err(e.into()),
                Err(SellError::Wallet(e)) => return Err(e.into()),
                Err(e) => format!("Sale failed: {}", i18n_errors::sell_error(&e, lang)),
            };
            update(
                ctx,
                c,
                embeds::notice("Sell to NPC", &text),
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

fn parse_amount_or_max(raw: &str, max_bal: Decimal) -> Option<Decimal> {
    if raw.eq_ignore_ascii_case("all") || raw.eq_ignore_ascii_case("max") {
        Some(max_bal)
    } else {
        let normalized = if raw.contains(',') && !raw.contains('.') {
            raw.replace(',', ".")
        } else {
            raw.to_string()
        };
        normalized.parse::<Decimal>().ok().map(|d| d.round_dp(2))
    }
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

    let lang = Lang::En;
    match action {
        "citydepmodal" => {
            let id = owner as i64;
            let raw = modal_field(m, "amount");
            let bal = economy::balance(&app.pool, id).await?;
            let note = match parse_amount_or_max(raw, bal) {
                Some(amount) if amount > Decimal::ZERO => {
                    let key = format!("citydep:{}", m.id.get());
                    match city::deposit_to_treasury(&app.pool, id, amount, &key).await {
                        Ok((wallet, treasury)) => format!(
                            "Deposited {} into City Treasury. Treasury: {}, Wallet: {}.",
                            format_bac(amount),
                            format_bac(treasury),
                            format_bac(wallet)
                        ),
                        Err(city::CityError::Db(e)) => return Err(e.into()),
                        Err(city::CityError::Wallet(e)) => return Err(e.into()),
                        Err(e) => i18n_errors::city_error(&e, lang),
                    }
                }
                _ => "Invalid deposit amount. Enter a number greater than 0 (or `max`)."
                    .to_string(),
            };
            let (embed, rows) = build_city_view(&app.pool, id, lang, Some(note)).await?;
            let msg = CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(rows);
            m.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg))
                .await?;
            Ok(())
        }
        "citywdmodal" => {
            let id = owner as i64;
            let raw = modal_field(m, "amount");
            let ov = city::overview(&app.pool, id).await?;
            let note = match parse_amount_or_max(raw, ov.treasury) {
                Some(amount) if amount > Decimal::ZERO => {
                    let key = format!("citywd:{}", m.id.get());
                    match city::withdraw_from_treasury(&app.pool, id, amount, &key).await {
                        Ok((wallet, treasury)) => format!(
                            "Withdrew {} from City Treasury. Treasury: {}, Wallet: {}.",
                            format_bac(amount),
                            format_bac(treasury),
                            format_bac(wallet)
                        ),
                        Err(city::CityError::Db(e)) => return Err(e.into()),
                        Err(city::CityError::Wallet(e)) => return Err(e.into()),
                        Err(e) => i18n_errors::city_error(&e, lang),
                    }
                }
                _ => "Invalid withdraw amount. Enter a number greater than 0 (or `max`)."
                    .to_string(),
            };
            let (embed, rows) = build_city_view(&app.pool, id, lang, Some(note)).await?;
            let msg = CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(rows);
            m.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg))
                .await?;
            Ok(())
        }
        "craftmodal" => {
            let id = owner as i64;
            let recipe_id = rest;
            let raw_qty = modal_field(m, "qty");
            let note = if raw_qty.eq_ignore_ascii_case("max") || raw_qty.eq_ignore_ascii_case("all") {
                if let Some(recipe) = find_recipe(recipe_id) {
                    let stock = player_stock(&app.pool, id).await?;
                    let batch_qty = max_craftable_batches(recipe, &stock).max(1);
                    run_craft_batch_note(app, id, recipe.id, batch_qty, lang).await?
                } else {
                    i18n_errors::craft_error(&crafting::CraftError::UnknownRecipe, lang)
                }
            } else {
                match raw_qty.parse::<i64>() {
                    Ok(n) if n > 0 => run_craft_batch_note(app, id, recipe_id, n, lang).await?,
                    _ => "Invalid quantity. Enter a whole number greater than 0 (or `max`)."
                        .to_string(),
                }
            };
            let (embed, rows) = build_craft_view(&app.pool, id, lang, Some(note)).await?;
            let msg = CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(rows);
            m.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg))
                .await?;
            Ok(())
        }
        "sellmodal" => {
            let item_id = rest;
            let raw_qty = modal_field(m, "qty");

            let text = match raw_qty.parse::<i64>() {
                Ok(n) if n > 0 => {
                    let key = format!("sell:{}", m.id.get());
                    match economy::sell_to_npc(&app.pool, owner as i64, item_id, SellQty::Exact(n), &key).await {
                        Ok(SaleOutcome::Sold { item, qty, total, balance }) => {
                            let it_name = item_name(item.id, lang);
                            format!(
                                "Sold {} {} for {}. Balance: {}.",
                                qty,
                                it_name,
                                format_bac(total),
                                format_bac(balance)
                            )
                        }
                        Ok(SaleOutcome::Duplicate) => "This sale has already been processed.".to_string(),
                        Err(SellError::Db(e)) => return Err(e.into()),
                        Err(SellError::Wallet(e)) => return Err(e.into()),
                        Err(e) => format!("Sale failed: {}", i18n_errors::sell_error(&e, lang)),
                    }
                }
                _ => "Invalid quantity. Enter a whole number greater than 0.".to_string(),
            };

            let msg = CreateInteractionResponseMessage::new()
                .embed(embeds::notice("Sell to NPC", &text))
                .components(components::main_menu(owner, lang));
            m.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg))
                .await?;
            Ok(())
        }
        _ => Ok(()),
    }
}
