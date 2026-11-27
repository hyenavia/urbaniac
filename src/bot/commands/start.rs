use crate::application::player;
use crate::bot::{components, embeds, AppState};
use serenity::all::*;

pub async fn run(ctx: &Context, cmd: &CommandInteraction, app: &AppState) -> anyhow::Result<()> {
    let user = &cmd.user;
    let name = user.global_name.clone().unwrap_or_else(|| user.name.clone());
    let avatar = user.face();
    let id = user.id.get() as i64;

    let (_, created) = player::register(&app.pool, id, &name, Some(&avatar)).await?;
    let profile = player::get_player_profile(&app.pool, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("profil hilang setelah registrasi"))?;

    let lang = profile.player.language();
    let msg = CreateInteractionResponseMessage::new()
        .embed(embeds::welcome(&profile, created, lang))
        .add_file(crate::bot::branding::banner_attachment())
        .components(components::profile_menu(user.id.get(), lang))
        .ephemeral(true);
    cmd.create_response(&ctx.http, CreateInteractionResponse::Message(msg)).await?;
    Ok(())
}
