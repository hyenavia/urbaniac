use crate::bot::{components, embeds, AppState};
use serenity::all::*;

pub async fn run(ctx: &Context, cmd: &CommandInteraction, app: &AppState) -> anyhow::Result<()> {
    let authorized = match app.bot_owner_id {
        Some(owner_id) => cmd.user.id.get() == owner_id,
        None => match cmd.guild_id {
            Some(gid) => gid
                .to_partial_guild(&ctx.http)
                .await
                .map(|g| g.owner_id == cmd.user.id)
                .unwrap_or(false),
            None => false,
        },
    };
    if !authorized {
        let msg = CreateInteractionResponseMessage::new()
            .content("🔒 Only the bot owner can run `/setup`.")
            .ephemeral(true);
        cmd.create_response(&ctx.http, CreateInteractionResponse::Message(msg))
            .await?;
        return Ok(());
    }

    let target_channel = cmd
        .data
        .options
        .iter()
        .find(|o| o.name == "channel")
        .and_then(|o| o.value.as_channel_id())
        .unwrap_or(cmd.channel_id);

    let panel = CreateMessage::new()
        .embed(embeds::host_panel())
        .add_file(crate::bot::branding::banner_attachment())
        .components(components::host_play_button());

    if let Err(e) = target_channel.send_message(&ctx.http, panel).await {
        tracing::warn!(channel = target_channel.get(), error = ?e, "failed to send setup panel");
        let err_msg = CreateInteractionResponseMessage::new()
            .content(format!(
                "⚠️ Failed to post panel to <#{}>. Make sure I have permission to send messages and attach files there.",
                target_channel.get()
            ))
            .ephemeral(true);
        cmd.create_response(&ctx.http, CreateInteractionResponse::Message(err_msg))
            .await?;
        return Ok(());
    }

    let reply = CreateInteractionResponseMessage::new()
        .content(format!("✅ Play Urbaniac panel posted to <#{}>.", target_channel.get()))
        .ephemeral(true);
    cmd.create_response(&ctx.http, CreateInteractionResponse::Message(reply))
        .await?;
    Ok(())
}
