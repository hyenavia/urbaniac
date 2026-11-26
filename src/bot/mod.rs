pub mod commands;
pub mod components;
pub mod embeds;
pub mod interactions;

use crate::config::Config;
use serenity::all::*;
use serenity::async_trait;
use sqlx::PgPool;

pub struct AppState {
    pub pool: PgPool,
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

pub async fn run(cfg: Config, pool: PgPool) -> anyhow::Result<()> {
    let handler = Handler { app: AppState { pool }, guild_id: cfg.guild_id };
    let mut client = Client::builder(&cfg.discord_token, GatewayIntents::empty())
        .event_handler(handler)
        .await?;
    client.start().await?;
    Ok(())
}
