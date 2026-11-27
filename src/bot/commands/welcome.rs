use crate::application::server;
use crate::bot::{branding, server_ui, AppState};
use serenity::all::*;

pub async fn run(ctx: &Context, cmd: &CommandInteraction, app: &AppState) -> anyhow::Result<()> {
    let Some(guild_id) = cmd.guild_id else {
        let msg = CreateInteractionResponseMessage::new()
            .content("This command can only be used inside a server.")
            .ephemeral(true);
        cmd.create_response(&ctx.http, CreateInteractionResponse::Message(msg))
            .await?;
        return Ok(());
    };

    let Some(sub) = cmd.data.options.first() else {
        return Ok(());
    };
    let CommandDataOptionValue::SubCommand(opts) = &sub.value else {
        return Ok(());
    };

    match sub.name.as_str() {
        "setup" => {
            let channel_id = opts
                .iter()
                .find_map(|o| match (&*o.name, &o.value) {
                    ("channel", CommandDataOptionValue::Channel(id)) => Some(id.get()),
                    _ => None,
                })
                .ok_or_else(|| anyhow::anyhow!("channel option missing"))?;
            let role_id = opts
                .iter()
                .find_map(|o| match (&*o.name, &o.value) {
                    ("role", CommandDataOptionValue::Role(id)) => Some(id.get()),
                    _ => None,
                })
                .unwrap_or(0);

            let existing = server::get_welcome_cached(&app.welcome_cache, guild_id.get()).await;
            let modal = server_ui::welcome_setup_modal(
                cmd.user.id.get(),
                channel_id,
                role_id,
                existing.as_ref(),
            );
            cmd.create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
                .await?;
        }
        "status" => {
            let cfg = server::get_welcome_cached(&app.welcome_cache, guild_id.get()).await;
            let embed = match cfg {
                Some(c) => {
                    let status = if c.enabled { "Enabled" } else { "Disabled" };
                    let role = c
                        .auto_role_id
                        .map(|r| format!("<@&{r}>"))
                        .unwrap_or_else(|| "None".to_string());
                    branding::styled_embed(branding::COLOR_SYSTEM)
                        .title("👋 Welcome Greet Configuration")
                        .field("Status", status, true)
                        .field("Channel", format!("<#{}>", c.channel_id), true)
                        .field("Auto-Role", role, true)
                        .field("Title", c.title, false)
                        .field("Message", c.message, false)
                }
                None => branding::styled_embed(branding::COLOR_SYSTEM)
                    .title("👋 Welcome Greet Configuration")
                    .description("Welcome greet is not configured yet. Run `/welcome setup` to configure it."),
            };
            let msg = CreateInteractionResponseMessage::new()
                .embed(embed)
                .ephemeral(true);
            cmd.create_response(&ctx.http, CreateInteractionResponse::Message(msg))
                .await?;
        }
        "disable" => {
            let updated = server::disable_welcome(&app.pool, &app.welcome_cache, guild_id.get()).await?;
            let text = if updated {
                "Welcome greet and auto-role have been disabled for this server."
            } else {
                "Welcome greet was not configured on this server."
            };
            let msg = CreateInteractionResponseMessage::new()
                .content(text)
                .ephemeral(true);
            cmd.create_response(&ctx.http, CreateInteractionResponse::Message(msg))
                .await?;
        }
        "test" => {
            let Some(cfg) = server::get_welcome_cached(&app.welcome_cache, guild_id.get()).await else {
                let msg = CreateInteractionResponseMessage::new()
                    .content("Welcome greet is not configured yet. Run `/welcome setup` first.")
                    .ephemeral(true);
                cmd.create_response(&ctx.http, CreateInteractionResponse::Message(msg))
                    .await?;
                return Ok(());
            };

            let (server_name, member_count) =
                if server::needs_guild_lookup(&cfg.title, &cfg.message) {
                    match guild_id.to_partial_guild_with_counts(&ctx.http).await {
                        Ok(g) => (g.name, g.approximate_member_count),
                        Err(_) => ("this server".to_string(), None),
                    }
                } else {
                    ("this server".to_string(), None)
                };

            let username = cmd
                .user
                .global_name
                .as_deref()
                .unwrap_or(cmd.user.name.as_str());
            let title = server::render_welcome_template(
                &cfg.title,
                cmd.user.id.get(),
                username,
                &server_name,
                member_count,
            );
            let message = server::render_welcome_template(
                &cfg.message,
                cmd.user.id.get(),
                username,
                &server_name,
                member_count,
            );
            let avatar = cmd.user.face();
            let role_note = cfg
                .auto_role_id
                .map(|r| format!(" (Auto-Role: <@&{r}>)"))
                .unwrap_or_default();

            let msg = CreateInteractionResponseMessage::new()
                .content(format!(
                    "**Preview** -> target channel <#{}>{role_note}:",
                    cfg.channel_id
                ))
                .embed(server_ui::welcome_embed(&title, &message, &avatar))
                .ephemeral(true);
            cmd.create_response(&ctx.http, CreateInteractionResponse::Message(msg))
                .await?;
        }
        _ => {}
    }

    Ok(())
}
