use crate::bot::{embeds, AppState};
use crate::domain::i18n::Lang;
use serenity::all::*;

pub async fn run(ctx: &Context, cmd: &CommandInteraction, _app: &AppState) -> anyhow::Result<()> {
    let msg = CreateInteractionResponseMessage::new()
        .embed(embeds::help(Lang::En))
        .add_file(crate::bot::branding::banner_attachment())
        .ephemeral(true);
    cmd.create_response(&ctx.http, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}
