pub mod branding;
pub mod commands;
pub mod components;
pub mod embeds;
pub mod i18n_errors;
pub mod interactions;
pub mod server_ui;

use crate::application::server::{self, WelcomeCache};
use crate::config::Config;
use serenity::all::*;
use serenity::async_trait;
use sqlx::PgPool;
use std::sync::{Arc, OnceLock};
use tokio::sync::Notify;

pub struct AppState {
    pub pool: PgPool,
    /// Membangunkan scheduler saat ada job baru.
    pub notify: Arc<Notify>,
    pub activity_duration_pct: i64,
    pub welcome_cache: WelcomeCache,
}

struct Handler {
    app: AppState,
    guild_id: Option<u64>,
    http_slot: Arc<OnceLock<Arc<Http>>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!(bot = %ready.user.name, "bot terhubung");
        let _ = self.http_slot.set(ctx.http.clone());
        ctx.set_activity(Some(ActivityData::custom(branding::BOT_STATUS)));
        let defs = commands::definitions();
        let result = match self.guild_id {
            Some(g) => GuildId::new(g).set_commands(&ctx.http, defs).await.map(|_| ()),
            None => Command::set_global_commands(&ctx.http, defs).await.map(|_| ()),
        };
        if let Err(e) = result {
            tracing::error!(error = ?e, "gagal mendaftarkan slash command");
        }
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        if new_member.user.bot {
            return;
        }
        let guild_id = new_member.guild_id.get();
        let Some(cfg) = server::get_welcome_cached(&self.app.welcome_cache, guild_id).await else {
            return;
        };
        if !cfg.enabled {
            return;
        }

        if let Some(role_id) = cfg.auto_role_id {
            if let Err(e) = new_member
                .add_role(&ctx.http, RoleId::new(role_id as u64))
                .await
            {
                tracing::warn!(guild_id, user_id = new_member.user.id.get(), role_id, error = ?e, "failed to assign auto-role");
            }
        }

        let (server_name, member_count) =
            if server::needs_guild_lookup(&cfg.title, &cfg.message) {
                match new_member.guild_id.to_partial_guild_with_counts(&ctx.http).await {
                    Ok(g) => (g.name, g.approximate_member_count),
                    Err(_) => ("this server".to_string(), None),
                }
            } else {
                ("this server".to_string(), None)
            };

        let user_id = new_member.user.id.get();
        let username = new_member
            .user
            .global_name
            .as_deref()
            .unwrap_or(new_member.user.name.as_str());
        let title = server::render_welcome_template(
            &cfg.title,
            user_id,
            username,
            &server_name,
            member_count,
        );
        let message = server::render_welcome_template(
            &cfg.message,
            user_id,
            username,
            &server_name,
            member_count,
        );
        let avatar = new_member.user.face();

        let channel = ChannelId::new(cfg.channel_id as u64);
        let msg = CreateMessage::new()
            .content(format!("<@{user_id}>"))
            .embed(server_ui::welcome_embed(&title, &message, &avatar));
        if let Err(e) = channel.send_message(&ctx.http, msg).await {
            tracing::warn!(guild_id, channel_id = cfg.channel_id, error = ?e, "failed to send welcome message");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let result = match &interaction {
            Interaction::Command(c) => interactions::handle_command(&ctx, c, &self.app).await,
            Interaction::Component(c) => interactions::handle_component(&ctx, c, &self.app).await,
            Interaction::Modal(m) => interactions::handle_modal(&ctx, m, &self.app).await,
            _ => Ok(()),
        };
        if let Err(e) = result {
            tracing::error!(error = ?e, "interaksi gagal");
        }
    }
}

pub async fn run(
    cfg: Config,
    pool: PgPool,
    notify: Arc<Notify>,
    http_slot: Arc<OnceLock<Arc<Http>>>,
) -> anyhow::Result<()> {
    let welcome_cache = server::load_welcome_cache(&pool).await?;
    let app = AppState {
        pool,
        notify,
        activity_duration_pct: cfg.activity_duration_pct,
        welcome_cache,
    };
    let handler = Handler { app, guild_id: cfg.guild_id, http_slot };
    let mut client = Client::builder(&cfg.discord_token, GatewayIntents::GUILD_MEMBERS)
        .event_handler(handler)
        .await?;
    client.start().await?;
    Ok(())
}
