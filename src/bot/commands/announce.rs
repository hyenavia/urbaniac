use crate::bot::{server_ui, AppState};
use serenity::all::*;

pub async fn run(ctx: &Context, cmd: &CommandInteraction, _app: &AppState) -> anyhow::Result<()> {
    if cmd.guild_id.is_none() {
        let msg = CreateInteractionResponseMessage::new()
            .content("This command can only be used inside a server.")
            .ephemeral(true);
        cmd.create_response(&ctx.http, CreateInteractionResponse::Message(msg))
            .await?;
        return Ok(());
    }

    let channel_id = cmd
        .data
        .options
        .iter()
        .find_map(|o| match (&*o.name, &o.value) {
            ("channel", CommandDataOptionValue::Channel(id)) => Some(id.get()),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("channel option missing"))?;

    let role_id = cmd
        .data
        .options
        .iter()
        .find_map(|o| match (&*o.name, &o.value) {
            ("mention", CommandDataOptionValue::Role(id)) => Some(id.get()),
            _ => None,
        })
        .unwrap_or(0);

    let ping_everyone = cmd
        .data
        .options
        .iter()
        .find_map(|o| match (&*o.name, &o.value) {
            ("everyone", CommandDataOptionValue::Boolean(b)) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);

    let modal = server_ui::announce_modal(cmd.user.id.get(), channel_id, role_id, ping_everyone);
    cmd.create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
        .await?;
    Ok(())
}
