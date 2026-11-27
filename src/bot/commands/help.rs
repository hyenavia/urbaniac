use crate::bot::{embeds, AppState};
use serenity::all::*;

pub async fn run(ctx: &Context, cmd: &CommandInteraction, _app: &AppState) -> anyhow::Result<()> {
    let msg = CreateInteractionResponseMessage::new()
        .embed(embeds::help())
        .add_file(crate::bot::branding::banner_attachment())
        .ephemeral(true);
    cmd.create_response(&ctx.http, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}
