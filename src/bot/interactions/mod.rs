use crate::application::{inventory, player};
use crate::bot::{commands, components, embeds, AppState};
use crate::domain::resources::find_item;
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

pub async fn handle_component(
    ctx: &Context,
    c: &ComponentInteraction,
    app: &AppState,
) -> anyhow::Result<()> {
    let Some((action, owner)) = components::parse_custom_id(&c.data.custom_id) else {
        return Ok(()); // bukan komponen milik kita
    };
    // Validasi pemilik data.
    if c.user.id.get() != owner {
        return deny(ctx, c, "Menu ini milik pemain lain. Jalankan /start untuk menu milikmu.").await;
    }
    let id = owner as i64;
    let embed = match action {
        "profile" => match player::get_player_profile(&app.pool, id).await? {
            Some(p) => embeds::profile(&p),
            None => return deny(ctx, c, "Profil belum ada. Jalankan /start.").await,
        },
        "inventory" => embeds::inventory(&inventory::view(&app.pool, id).await?, None),
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
            embeds::inventory(&inventory::view(&app.pool, id).await?, Some(note))
        }
        _ => return Ok(()),
    };
    let msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(components::main_menu(owner));
    c.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg)).await?;
    Ok(())
}
