use crate::bot::{components, embeds, AppState};
use serenity::all::*;

pub async fn run(ctx: &Context, cmd: &CommandInteraction, _app: &AppState) -> anyhow::Result<()> {
    let msg = CreateInteractionResponseMessage::new()
        .embed(embeds::host_panel())
        .add_file(crate::bot::branding::banner_attachment())
        .components(components::host_play_button());
    cmd.create_response(&ctx.http, CreateInteractionResponse::Message(msg))
        .await?;
    Ok(())
}
