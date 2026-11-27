use crate::application::player;
use crate::bot::{embeds, AppState};
use crate::domain::i18n::Lang;
use serenity::all::*;

pub async fn run(ctx: &Context, cmd: &CommandInteraction, app: &AppState) -> anyhow::Result<()> {
    let lang = player::get_lang(&app.pool, cmd.user.id.get() as i64)
        .await
        .unwrap_or(Lang::En);
    let msg = CreateInteractionResponseMessage::new()
        .embed(embeds::help(lang))
        .add_file(crate::bot::branding::banner_attachment())
        .ephemeral(true);
    cmd.create_response(&ctx.http, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}
