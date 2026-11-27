use crate::application::server::{self, KIND_CASH, KIND_STANDARD};
use crate::bot::{server_ui, AppState};
use crate::domain::economy::format_bac;
use rust_decimal::Decimal;
use serenity::all::*;

fn get_i64(opts: &[CommandDataOption], name: &str) -> Option<i64> {
    opts.iter().find_map(|o| match (&*o.name, &o.value) {
        (n, CommandDataOptionValue::Integer(v)) if n == name => Some(*v),
        _ => None,
    })
}

fn get_str<'a>(opts: &'a [CommandDataOption], name: &str) -> Option<&'a str> {
    opts.iter().find_map(|o| match (&*o.name, &o.value) {
        (n, CommandDataOptionValue::String(s)) if n == name => Some(s.as_str()),
        _ => None,
    })
}

fn get_channel(opts: &[CommandDataOption], name: &str) -> Option<ChannelId> {
    opts.iter().find_map(|o| match (&*o.name, &o.value) {
        (n, CommandDataOptionValue::Channel(id)) if n == name => Some(*id),
        _ => None,
    })
}

async fn reply_ephemeral(
    ctx: &Context,
    cmd: &CommandInteraction,
    text: &str,
) -> anyhow::Result<()> {
    let msg = CreateInteractionResponseMessage::new()
        .content(text)
        .ephemeral(true);
    cmd.create_response(&ctx.http, CreateInteractionResponse::Message(msg))
        .await?;
    Ok(())
}

async fn launch_giveaway(
    ctx: &Context,
    cmd: &CommandInteraction,
    app: &AppState,
    guild_id: u64,
    channel_id: ChannelId,
    kind: &str,
    prize: &str,
    cash_amount: Decimal,
    winners: i32,
    duration_minutes: i64,
) -> anyhow::Result<()> {
    cmd.defer_ephemeral(&ctx.http).await?;
    let gw = match server::start_giveaway(
        &app.pool,
        &app.notify,
        guild_id,
        channel_id.get(),
        cmd.user.id.get(),
        kind,
        prize,
        cash_amount,
        winners,
        duration_minutes,
    )
    .await
    {
        Ok(gw) => gw,
        Err(e) => {
            cmd.edit_response(&ctx.http, EditInteractionResponse::new().content(format!("Failed: {e}")))
                .await?;
            return Ok(());
        }
    };

    let msg = CreateMessage::new()
        .embed(server_ui::giveaway_active_embed(&gw, 0))
        .components(server_ui::giveaway_join_button(gw.id, 0, false));
    match channel_id.send_message(&ctx.http, msg).await {
        Ok(sent) => {
            server::attach_message_id(&app.pool, gw.id, sent.id.get()).await?;
            cmd.edit_response(
                &ctx.http,
                EditInteractionResponse::new().content(format!(
                    "🎉 Giveaway **#{}** started in <#{}>!",
                    gw.id,
                    channel_id.get()
                )),
            )
            .await?;
        }
        Err(e) => {
            cmd.edit_response(
                &ctx.http,
                EditInteractionResponse::new().content(format!(
                    "Giveaway #{} was created, but failed to post in <#{}>: {e}",
                    gw.id,
                    channel_id.get()
                )),
            )
            .await?;
        }
    }
    Ok(())
}

pub async fn run_giveaway(
    ctx: &Context,
    cmd: &CommandInteraction,
    app: &AppState,
) -> anyhow::Result<()> {
    let Some(guild_id) = cmd.guild_id else {
        return reply_ephemeral(ctx, cmd, "This command can only be used inside a server.").await;
    };
    let Some(sub) = cmd.data.options.first() else {
        return Ok(());
    };
    let CommandDataOptionValue::SubCommand(opts) = &sub.value else {
        return Ok(());
    };

    match sub.name.as_str() {
        "start" => {
            let prize = get_str(opts, "prize").unwrap_or("Prize").trim();
            let duration = get_i64(opts, "duration_minutes").unwrap_or(60);
            let winners = get_i64(opts, "winners").unwrap_or(1) as i32;
            let channel_id = get_channel(opts, "channel").unwrap_or(cmd.channel_id);
            launch_giveaway(
                ctx,
                cmd,
                app,
                guild_id.get(),
                channel_id,
                KIND_STANDARD,
                prize,
                Decimal::ZERO,
                winners,
                duration,
            )
            .await
        }
        "end" => {
            let Some(id) = get_i64(opts, "id") else {
                return Ok(());
            };
            cmd.defer_ephemeral(&ctx.http).await?;
            let text = match server::end_giveaway_now(&app.pool, id).await? {
                Some(res) => {
                    server_ui::publish_giveaway_result(&ctx.http, &res).await;
                    format!("Giveaway **#{id}** ended and winners have been announced.")
                }
                None => format!("Giveaway **#{id}** was not found or has already ended."),
            };
            cmd.edit_response(&ctx.http, EditInteractionResponse::new().content(text))
                .await?;
            Ok(())
        }
        "reroll" => {
            let Some(id) = get_i64(opts, "id") else {
                return Ok(());
            };
            cmd.defer_ephemeral(&ctx.http).await?;
            let text = match server::reroll_giveaway(&app.pool, id, cmd.id.get()).await? {
                Some(res) => {
                    server_ui::publish_giveaway_result(&ctx.http, &res).await;
                    format!("Giveaway **#{id}** rerolled and the new winner has been announced.")
                }
                None => format!("Giveaway **#{id}** was not found or has not ended yet."),
            };
            cmd.edit_response(&ctx.http, EditInteractionResponse::new().content(text))
                .await?;
            Ok(())
        }
        _ => Ok(()),
    }
}

pub async fn run_cash_giveaway(
    ctx: &Context,
    cmd: &CommandInteraction,
    app: &AppState,
) -> anyhow::Result<()> {
    let Some(guild_id) = cmd.guild_id else {
        return reply_ephemeral(ctx, cmd, "This command can only be used inside a server.").await;
    };
    let opts = &cmd.data.options;
    let amount = get_i64(opts, "amount").unwrap_or(0);
    if amount <= 0 {
        return reply_ephemeral(ctx, cmd, "Cash amount must be greater than 0.").await;
    }
    let duration = get_i64(opts, "duration_minutes").unwrap_or(60);
    let winners = get_i64(opts, "winners").unwrap_or(1) as i32;
    let channel_id = get_channel(opts, "channel").unwrap_or(cmd.channel_id);
    let cash_amount = Decimal::from(amount);
    let prize = match get_str(opts, "note").map(str::trim).filter(|s| !s.is_empty()) {
        Some(note) => format!("{} — {note}", format_bac(cash_amount)),
        None => format_bac(cash_amount),
    };

    launch_giveaway(
        ctx,
        cmd,
        app,
        guild_id.get(),
        channel_id,
        KIND_CASH,
        &prize,
        cash_amount,
        winners,
        duration,
    )
    .await
}
