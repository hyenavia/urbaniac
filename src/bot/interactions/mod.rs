use crate::application::economy::{self, SaleOutcome, SellError, SellQty};
use crate::application::{activities, city, inventory, player};
use crate::bot::{commands, components, embeds, AppState};
use crate::domain::activities::ActivityKind;
use crate::domain::city::ProjectId;
use crate::domain::economy::format_bac;
use crate::domain::resources::find_item;
use rust_decimal::Decimal;
use serenity::all::*;

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
            let items: Vec<(String, String, i64)> = v
                .lines
                .iter()
                .map(|l| (l.item.name.to_string(), l.item.id.to_string(), l.quantity))
                .collect();
            update(ctx, c, embeds::sell_prompt(&v), components::sell_select(owner, &items)).await
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
            update(ctx, c, embeds::sell_item(item, owned), components::sell_qty(owner, item.id)).await
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
