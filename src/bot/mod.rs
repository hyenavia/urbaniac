pub mod branding;
pub mod commands;
pub mod components;
pub mod embeds;
pub mod giveaway_ui;
pub mod i18n_errors;
pub mod interactions;

use crate::application::voice_boost::{
    VoiceBoostTracker, VOICE_BOOST_SPEED_PCT, VOICE_BOOST_XP_PCT,
};
use crate::config::Config;
use crate::web::events::{EventHub, RealtimeEvent};
use serenity::all::*;
use serenity::async_trait;
use sqlx::PgPool;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::time::MissedTickBehavior;

pub struct AppState {
    pub pool: PgPool,
    /// Membangunkan scheduler saat ada job baru.
    pub notify: Arc<Notify>,
    pub activity_duration_pct: i64,
    pub bot_owner_id: Option<u64>,
    pub voice_boost: Arc<VoiceBoostTracker>,
    pub events: Option<Arc<EventHub>>,
}

pub fn format_uptime(elapsed: Duration) -> String {
    let total_mins = elapsed.as_secs() / 60;
    let hours = total_mins / 60;
    let mins = total_mins % 60;
    let hour_label = if hours == 1 { "hour" } else { "hours" };
    let min_label = if mins == 1 { "min" } else { "mins" };
    format!("Uptime: {hours} {hour_label} {mins} {min_label}")
}

struct Handler {
    app: AppState,
    guild_id: Option<u64>,
    activity_enabled: bool,
    http_slot: Arc<OnceLock<Arc<Http>>>,
    started_at: Instant,
    uptime_task_started: AtomicBool,
}

impl Handler {
    async fn sync_commands(&self, http: &Arc<Http>) -> serenity::Result<()> {
        let defs = commands::definitions();
        let global_commands = match self.guild_id {
            Some(g) => {
                let guild = GuildId::new(g);
                let existing = guild.get_commands(http).await?;
                for cmd in existing {
                    if cmd.kind == CommandType::ChatInput
                        && !commands::NAMES.contains(&cmd.name.as_str())
                    {
                        guild.delete_command(http, cmd.id).await?;
                    }
                }
                for def in defs {
                    guild.create_command(http, def).await?;
                }
                // Pastikan tidak ada ChatInput command global tersisa saat bot dikunci ke server resmi.
                let globals = Command::get_global_commands(http).await?;
                for cmd in &globals {
                    if cmd.kind == CommandType::ChatInput {
                        Command::delete_global_command(http, cmd.id).await?;
                    }
                }
                if self.activity_enabled {
                    Some(globals)
                } else {
                    None
                }
            }
            None => {
                let existing = Command::get_global_commands(http).await?;
                for cmd in &existing {
                    if cmd.kind == CommandType::ChatInput
                        && !commands::NAMES.contains(&cmd.name.as_str())
                    {
                        Command::delete_global_command(http, cmd.id).await?;
                    }
                }
                for def in defs {
                    Command::create_global_command(http, def).await?;
                }
                if self.activity_enabled {
                    Some(existing)
                } else {
                    None
                }
            }
        };

        if let Some(globals) = global_commands {
            let integration_types = vec![InstallationContext::Guild, InstallationContext::User];
            let contexts = vec![
                InteractionContext::Guild,
                InteractionContext::BotDm,
                InteractionContext::PrivateChannel,
            ];
            let existing_ep = globals
                .iter()
                .find(|cmd| cmd.kind == CommandType::PrimaryEntryPoint);
            let needs_sync = match existing_ep {
                None => true,
                Some(ep) => {
                    ep.handler != Some(EntryPointHandlerType::AppHandler)
                        || ep.integration_types != integration_types
                        || ep.contexts.as_ref() != Some(&contexts)
                }
            };
            if needs_sync {
                let launch = CreateCommand::new("launch")
                    .description("Launch the Urbaniac visual city Activity")
                    .kind(CommandType::PrimaryEntryPoint)
                    .handler(EntryPointHandlerType::AppHandler)
                    .integration_types(integration_types)
                    .contexts(contexts);
                let res = match existing_ep {
                    Some(ep) => Command::edit_global_command(http, ep.id, launch).await,
                    None => Command::create_global_command(http, launch).await,
                };
                if let Err(e) = res {
                    tracing::warn!(error = ?e, "failed to sync PrimaryEntryPoint command");
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!(bot = %ready.user.name, "bot terhubung");
        let _ = self.http_slot.set(ctx.http.clone());
        ctx.set_activity(Some(ActivityData::custom(format_uptime(
            self.started_at.elapsed(),
        ))));
        if !self.uptime_task_started.swap(true, Ordering::SeqCst) {
            let started_at = self.started_at;
            let status_ctx = ctx.clone();
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(60));
                ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
                ticker.tick().await;
                loop {
                    ticker.tick().await;
                    status_ctx.set_activity(Some(ActivityData::custom(format_uptime(
                        started_at.elapsed(),
                    ))));
                }
            });
        }
        if let Err(e) = self.sync_commands(&ctx.http).await {
            tracing::error!(error = ?e, "gagal mendaftarkan slash command");
        }
    }

    async fn guild_create(&self, _ctx: Context, guild: Guild, _is_new: Option<bool>) {
        self.app.voice_boost.sync_guild_voice_states(
            guild.id.get(),
            guild
                .voice_states
                .iter()
                .map(|(uid, vs)| (uid.get(), vs.channel_id.map(|c| c.get()))),
        );
    }

    async fn voice_state_update(
        &self,
        _ctx: Context,
        _old: Option<VoiceState>,
        new: VoiceState,
    ) {
        let change = self.app.voice_boost.handle_voice_update(
            new.guild_id.map(|g| g.get()),
            new.user_id.get(),
            new.channel_id.map(|c| c.get()),
        );
        if let Some(active) = change {
            if let Some(hub) = &self.app.events {
                let msg = if active {
                    format!(
                        "🔊 Voice Boost active! (+{VOICE_BOOST_SPEED_PCT}% Activity & Craft Speed, +{VOICE_BOOST_XP_PCT}% XP)"
                    )
                } else {
                    "🔇 Voice Boost ended.".to_string()
                };
                hub.publish(RealtimeEvent {
                    player_id: Some(new.user_id.get() as i64),
                    kind: "voice_boost",
                    message: Some(msg),
                });
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Some(allowed) = self.guild_id {
            let allow_anywhere = match &interaction {
                Interaction::Command(c) => c.data.kind == CommandType::PrimaryEntryPoint,
                Interaction::Component(c) => c.data.custom_id == "ub:launch:0",
                _ => false,
            };
            if !allow_anywhere {
                let gid = match &interaction {
                    Interaction::Command(c) => c.guild_id,
                    Interaction::Component(c) => c.guild_id,
                    Interaction::Modal(m) => m.guild_id,
                    _ => None,
                };
                if gid != Some(GuildId::new(allowed)) {
                    let msg = CreateInteractionResponseMessage::new()
                        .content("🔒 Urbaniac bot commands are only available on the official server. Open the **Urbaniac Activity** to play anywhere!")
                        .components(components::dm_activity_button())
                        .ephemeral(true);
                    let resp = CreateInteractionResponse::Message(msg);
                    match &interaction {
                        Interaction::Command(c) => {
                            let _ = c.create_response(&ctx.http, resp).await;
                        }
                        Interaction::Component(c) => {
                            let _ = c.create_response(&ctx.http, resp).await;
                        }
                        Interaction::Modal(m) => {
                            let _ = m.create_response(&ctx.http, resp).await;
                        }
                        _ => {}
                    }
                    return;
                }
            }
        }

        let result = match &interaction {
            Interaction::Command(c) => interactions::handle_command(&ctx, c, &self.app).await,
            Interaction::Component(c) => interactions::handle_component(&ctx, c, &self.app).await,
            Interaction::Modal(m) => interactions::handle_modal(&ctx, m, &self.app).await,
            _ => Ok(()),
        };
        if let Err(e) = result {
            tracing::error!(error = ?e, "interaksi gagal");
            let msg = CreateInteractionResponseMessage::new()
                .content("⚠️ Something went wrong while processing your action. Please try again.")
                .ephemeral(true);
            let resp = CreateInteractionResponse::Message(msg);
            match &interaction {
                Interaction::Command(c) => {
                    let _ = c.create_response(&ctx.http, resp).await;
                }
                Interaction::Component(c) => {
                    let _ = c.create_response(&ctx.http, resp).await;
                }
                Interaction::Modal(m) => {
                    let _ = m.create_response(&ctx.http, resp).await;
                }
                _ => {}
            }
        }
    }
}

pub async fn run(
    cfg: Config,
    pool: PgPool,
    notify: Arc<Notify>,
    http_slot: Arc<OnceLock<Arc<Http>>>,
    voice_boost: Arc<VoiceBoostTracker>,
    events: Option<Arc<EventHub>>,
) -> anyhow::Result<()> {
    let app = AppState {
        pool,
        notify,
        activity_duration_pct: cfg.activity_duration_pct,
        bot_owner_id: cfg.bot_owner_id,
        voice_boost,
        events,
    };
    let handler = Handler {
        app,
        guild_id: cfg.guild_id,
        activity_enabled: cfg.activity.enabled,
        http_slot,
        started_at: Instant::now(),
        uptime_task_started: AtomicBool::new(false),
    };
    let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_VOICE_STATES;
    let mut client = Client::builder(&cfg.discord_token, intents)
        .event_handler(handler)
        .await?;
    client.start().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_uptime_status() {
        assert_eq!(format_uptime(Duration::from_secs(0)), "Uptime: 0 hours 0 mins");
        assert_eq!(format_uptime(Duration::from_secs(3_660)), "Uptime: 1 hour 1 min");
        assert_eq!(
            format_uptime(Duration::from_secs((145 * 60 + 38) * 60)),
            "Uptime: 145 hours 38 mins"
        );
    }
}
