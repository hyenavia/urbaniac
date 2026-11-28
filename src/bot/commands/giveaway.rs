use crate::application::giveaway::{self, GiveawayFinishResult};
use crate::bot::{giveaway_ui, AppState};
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

async fn post_giveaway_panel(
    ctx: &Context,
    cmd: &CommandInteraction,
    app: &AppState,
    channel_id: ChannelId,
    gw: crate::infrastructure::repositories::giveaway_repo::Giveaway,
) -> anyhow::Result<()> {
    let msg = CreateMessage::new()
        .embed(giveaway_ui::giveaway_active_embed(&gw, 0))
        .components(giveaway_ui::giveaway_join_button(gw.id, 0, false));
    match channel_id.send_message(&ctx.http, msg).await {
        Ok(sent) => {
            giveaway::attach_message_id(&app.pool, gw.id, sent.id.get()).await?;
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
            if let Err(rb_err) = giveaway::rollback_unposted(&app.pool, gw.id).await {
                tracing::warn!(giveaway_id = gw.id, error = ?rb_err, "failed to rollback unposted giveaway");
            }
            cmd.edit_response(
                &ctx.http,
                EditInteractionResponse::new().content(format!(
                    "⚠️ Failed to post giveaway in <#{}>: {e}",
                    channel_id.get()
                )),
            )
            .await?;
        }
    }
    Ok(())
}

async fn handle_finish_action(
    ctx: &Context,
    cmd: &CommandInteraction,
    result: Option<GiveawayFinishResult>,
    success_text: String,
    missing_text: String,
) -> anyhow::Result<()> {
    let text = match result {
        Some(res) => {
            giveaway_ui::publish_giveaway_result(&ctx.http, &res).await;
            success_text
        }
        None => missing_text,
    };
    cmd.edit_response(&ctx.http, EditInteractionResponse::new().content(text))
        .await?;
    Ok(())
}

pub async fn run(ctx: &Context, cmd: &CommandInteraction, app: &AppState) -> anyhow::Result<()> {
    let Some(guild_id) = cmd.guild_id else {
        return reply_ephemeral(ctx, cmd, "This command can only be used inside a server.").await;
    };

    let is_admin = cmd
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.administrator() || p.manage_guild())
        .unwrap_or(false);
    if !is_admin {
        return reply_ephemeral(
            ctx,
            cmd,
            "⛔ Only server administrators (`Manage Server` / `Administrator`) can use `/giveaway`.",
        )
        .await;
    }

    let Some(sub) = cmd.data.options.first() else {
        return Ok(());
    };
    let CommandDataOptionValue::SubCommand(opts) = &sub.value else {
        return Ok(());
    };

    match sub.name.as_str() {
        "cash" => {
            let amount = get_i64(opts, "amount").unwrap_or(0);
            let duration = get_i64(opts, "duration_minutes").unwrap_or(60);
            let winners = get_i64(opts, "winners").unwrap_or(1) as i32;
            let channel_id = get_channel(opts, "channel").unwrap_or(cmd.channel_id);
            let note = get_str(opts, "note");

            cmd.defer_ephemeral(&ctx.http).await?;
            match giveaway::start_cash_giveaway(
                &app.pool,
                &app.notify,
                guild_id.get(),
                channel_id.get(),
                cmd.user.id.get(),
                amount,
                winners,
                duration,
                note,
            )
            .await
            {
                Ok(gw) => post_giveaway_panel(ctx, cmd, app, channel_id, gw).await,
                Err(e) => {
                    cmd.edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content(format!("Failed: {e}")),
                    )
                    .await?;
                    Ok(())
                }
            }
        }
        "item" => {
            let item_id = get_str(opts, "item").unwrap_or("");
            let qty = get_i64(opts, "quantity").unwrap_or(0);
            let duration = get_i64(opts, "duration_minutes").unwrap_or(60);
            let winners = get_i64(opts, "winners").unwrap_or(1) as i32;
            let channel_id = get_channel(opts, "channel").unwrap_or(cmd.channel_id);
            let note = get_str(opts, "note");

            cmd.defer_ephemeral(&ctx.http).await?;
            match giveaway::start_item_giveaway(
                &app.pool,
                &app.notify,
                guild_id.get(),
                channel_id.get(),
                cmd.user.id.get(),
                item_id,
                qty,
                winners,
                duration,
                note,
            )
            .await
            {
                Ok(gw) => post_giveaway_panel(ctx, cmd, app, channel_id, gw).await,
                Err(e) => {
                    cmd.edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content(format!("Failed: {e}")),
                    )
                    .await?;
                    Ok(())
                }
            }
        }
        "end" => {
            let Some(id) = get_i64(opts, "id") else {
                return Ok(());
            };
            cmd.defer_ephemeral(&ctx.http).await?;
            let res = giveaway::end_giveaway_now(&app.pool, guild_id.get(), id).await?;
            handle_finish_action(
                ctx,
                cmd,
                res,
                format!("Giveaway **#{id}** ended and winners have been announced."),
                format!("Giveaway **#{id}** was not found in this server or has already ended."),
            )
            .await
        }
        "reroll" => {
            let Some(id) = get_i64(opts, "id") else {
                return Ok(());
            };
            cmd.defer_ephemeral(&ctx.http).await?;
            let res = giveaway::reroll_giveaway(&app.pool, guild_id.get(), id, cmd.id.get()).await?;
            handle_finish_action(
                ctx,
                cmd,
                res,
                format!("Giveaway **#{id}** rerolled and the new winner has been announced."),
                format!("Giveaway **#{id}** was not found in this server or has not ended yet."),
            )
            .await
        }
        _ => Ok(()),
    }
}
