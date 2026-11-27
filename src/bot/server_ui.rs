use crate::application::server::{GiveawayFinishResult, KIND_CASH};
use crate::bot::{branding, components};
use crate::domain::economy::format_bac;
use crate::infrastructure::repositories::server_repo::{Giveaway, WelcomeConfig};
use serenity::all::*;

pub fn welcome_embed(title: &str, message: &str, avatar_url: &str) -> CreateEmbed {
    branding::styled_embed(branding::COLOR_GENERAL)
        .title(title)
        .description(message)
        .thumbnail(avatar_url)
}

pub fn announcement_embed(
    title: &str,
    body: &str,
    image_url: Option<&str>,
    author_tag: &str,
) -> CreateEmbed {
    let mut e = branding::styled_embed(branding::COLOR_EXPLORATION)
        .title(title)
        .description(body)
        .footer(CreateEmbedFooter::new(format!(
            "Urbaniac • Announced by {author_tag}"
        )));
    if let Some(url) = image_url {
        e = e.image(url);
    }
    e
}

pub fn giveaway_active_embed(gw: &Giveaway, entries: i64) -> CreateEmbed {
    let ts = gw.ends_at.timestamp();
    let is_cash = gw.kind == KIND_CASH;
    let color = if is_cash {
        branding::COLOR_ECONOMY
    } else {
        branding::COLOR_SOCIAL
    };
    let header = if is_cash {
        "💰 Urbaniac Cash Giveaway"
    } else {
        "🎉 Community Giveaway"
    };

    let mut e = branding::styled_embed(color)
        .title(header)
        .field("Prize", format!("**{}**", gw.prize), false)
        .field("Winners", gw.winner_count.to_string(), true)
        .field("Entries", entries.to_string(), true)
        .field("Ends", format!("<t:{ts}:R> (<t:{ts}:F>)"), false)
        .field("Hosted by", format!("<@{}>", gw.host_id), true)
        .footer(CreateEmbedFooter::new(format!(
            "Urbaniac • Giveaway #{}",
            gw.id
        )));

    if is_cash {
        e = e.description(
            "Press **Join Giveaway** below! Requires an Urbaniac profile (`/start` or **Play**). Cash is credited automatically when the giveaway ends.",
        );
    } else {
        e = e.description("Press **Join Giveaway** below to enter!");
    }
    e
}

pub fn giveaway_ended_embed(res: &GiveawayFinishResult) -> CreateEmbed {
    let gw = &res.giveaway;
    let winners_text = if gw.winner_ids.is_empty() {
        "No valid entries — no winner selected.".to_string()
    } else {
        gw.winner_ids
            .iter()
            .map(|id| format!("<@{id}>"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let title = if gw.kind == KIND_CASH {
        "💰 Cash Giveaway Ended"
    } else {
        "🎉 Giveaway Ended"
    };

    branding::styled_embed(branding::COLOR_INFRASTRUCTURE)
        .title(title)
        .field("Prize", format!("**{}**", gw.prize), false)
        .field("Winner(s)", winners_text, false)
        .field("Total Entries", res.total_entries.to_string(), true)
        .field("Hosted by", format!("<@{}>", gw.host_id), true)
        .footer(CreateEmbedFooter::new(format!(
            "Urbaniac • Giveaway #{} • Ended",
            gw.id
        )))
}

pub fn giveaway_join_button(
    giveaway_id: i64,
    entries: i64,
    disabled: bool,
) -> Vec<CreateActionRow> {
    let label = if disabled {
        format!("Ended ({entries})")
    } else {
        format!("Join Giveaway ({entries})")
    };
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{}:gwjoin:0:{giveaway_id}", components::PREFIX))
            .label(label)
            .emoji(ReactionType::Unicode("🎉".to_string()))
            .style(if disabled {
                ButtonStyle::Secondary
            } else {
                ButtonStyle::Success
            })
            .disabled(disabled),
    ])]
}

pub fn welcome_setup_modal(
    owner: u64,
    channel_id: u64,
    role_id: u64,
    existing: Option<&WelcomeConfig>,
) -> CreateModal {
    let default_title = existing
        .map(|c| c.title.as_str())
        .unwrap_or("Welcome to {server}!");
    let default_message = existing.map(|c| c.message.as_str()).unwrap_or(
        "Hey {user}, welcome to **{server}**! You are member #{member_count}.\nUse `/start` or click **Play** to begin your Urbaniac journey!",
    );

    let title_input = CreateInputText::new(InputTextStyle::Short, "Welcome Embed Title", "title")
        .value(default_title)
        .placeholder("e.g. Welcome to {server}!")
        .required(true)
        .min_length(1)
        .max_length(120);

    let message_input = CreateInputText::new(
        InputTextStyle::Paragraph,
        "Message ({user}, {username}, {server}, {member_count})",
        "message",
    )
    .value(default_message)
    .placeholder("Write your welcome message here...")
    .required(true)
    .min_length(1)
    .max_length(1500);

    CreateModal::new(
        format!(
            "{}:welcomemodal:{owner}:{channel_id}:{role_id}",
            components::PREFIX
        ),
        "Configure Welcome Greet",
    )
    .components(vec![
        CreateActionRow::InputText(title_input),
        CreateActionRow::InputText(message_input),
    ])
}

pub fn announce_modal(
    owner: u64,
    channel_id: u64,
    role_id: u64,
    ping_everyone: bool,
) -> CreateModal {
    let title_input = CreateInputText::new(InputTextStyle::Short, "Announcement Title", "title")
        .placeholder("e.g. Server Maintenance / New Patch!")
        .required(true)
        .min_length(1)
        .max_length(200);

    let body_input =
        CreateInputText::new(InputTextStyle::Paragraph, "Announcement Body", "body")
            .placeholder("Write your announcement content (supports Discord markdown)...")
            .required(true)
            .min_length(1)
            .max_length(3500);

    let image_input = CreateInputText::new(
        InputTextStyle::Short,
        "Image URL (Optional)",
        "image_url",
    )
    .placeholder("https://...")
    .required(false)
    .max_length(300);

    CreateModal::new(
        format!(
            "{}:announcemodal:{owner}:{channel_id}:{role_id}:{}",
            components::PREFIX,
            if ping_everyone { 1 } else { 0 }
        ),
        "Create Server Announcement",
    )
    .components(vec![
        CreateActionRow::InputText(title_input),
        CreateActionRow::InputText(body_input),
        CreateActionRow::InputText(image_input),
    ])
}

pub async fn publish_giveaway_result(http: &Http, res: &GiveawayFinishResult) {
    let gw = &res.giveaway;
    let channel = ChannelId::new(gw.channel_id as u64);

    if gw.message_id != 0 {
        let msg_id = MessageId::new(gw.message_id as u64);
        let edit = EditMessage::new()
            .embed(giveaway_ended_embed(res))
            .components(giveaway_join_button(gw.id, res.total_entries, true));
        if let Err(e) = channel.edit_message(http, msg_id, edit).await {
            tracing::warn!(giveaway_id = gw.id, error = ?e, "failed to update giveaway message");
        }
    }

    let summary = if res.winners.is_empty() {
        format!(
            "🎉 **Giveaway #{} ({})** ended with no valid entries, so no winner could be selected.",
            gw.id, gw.prize
        )
    } else {
        let mentions = res
            .winners
            .iter()
            .map(|id| format!("<@{id}>"))
            .collect::<Vec<_>>()
            .join(", ");
        let prefix = if res.is_reroll {
            format!("🔁 **Giveaway #{} Reroll!** New winner: {mentions} — won **{}**!", gw.id, gw.prize)
        } else {
            format!("🎉 **Congratulations {mentions}!** You won **{}** (Giveaway #{})!", gw.prize, gw.id)
        };

        if gw.kind == KIND_CASH {
            let mut parts = vec![prefix];
            if !res.credited_winners.is_empty() {
                parts.push(format!(
                    "💰 **{}** has been credited automatically to the winner's Urbaniac wallet.",
                    format_bac(gw.cash_amount)
                ));
            }
            if !res.skipped_no_wallet.is_empty() {
                let missing = res
                    .skipped_no_wallet
                    .iter()
                    .map(|id| format!("<@{id}>"))
                    .collect::<Vec<_>>()
                    .join(", ");
                parts.push(format!(
                    "⚠️ Could not credit {missing} because they do not have an Urbaniac wallet yet."
                ));
            }
            parts.join("\n")
        } else {
            prefix
        }
    };

    let msg = CreateMessage::new().content(summary);
    if let Err(e) = channel.send_message(http, msg).await {
        tracing::warn!(giveaway_id = gw.id, error = ?e, "failed to send giveaway winner message");
    }
}
