pub mod commands;
pub mod components;
pub mod embeds;
pub mod interactions;

use crate::config::Config;
use serenity::all::*;
use serenity::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Notify;

pub struct AppState {
    pub pool: PgPool,
    /// Membangunkan scheduler saat ada job baru.
    pub notify: Arc<Notify>,
    pub activity_duration_pct: i64,
}

struct Handler {
    app: AppState,
    guild_id: Option<u64>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!(bot = %ready.user.name, "bot terhubung");
        let defs = commands::definitions();
        let result = match self.guild_id {
            Some(g) => GuildId::new(g).set_commands(&ctx.http, defs).await.map(|_| ()),
            None => Command::set_global_commands(&ctx.http, defs).await.map(|_| ()),
        };
        if let Err(e) = result {
            tracing::error!(error = ?e, "gagal mendaftarkan slash command");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let result = match &interaction {
            Interaction::Command(c) => interactions::handle_command(&ctx, c, &self.app).await,
            Interaction::Component(c) => interactions::handle_component(&ctx, c, &self.app).await,
            _ => Ok(()),
        };
        if let Err(e) = result {
            tracing::error!(error = ?e, "interaksi gagal");
        }
    }
}

pub async fn run(cfg: Config, pool: PgPool, notify: Arc<Notify>) -> anyhow::Result<()> {
    let app = AppState { pool, notify, activity_duration_pct: cfg.activity_duration_pct };
    let handler = Handler { app, guild_id: cfg.guild_id };
    let mut client = Client::builder(&cfg.discord_token, GatewayIntents::empty())
        .event_handler(handler)
        .await?;
    client.start().await?;
    Ok(())
}
