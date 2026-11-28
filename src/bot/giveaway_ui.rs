use crate::application::giveaway::{GiveawayFinishResult, KIND_CASH, KIND_ITEM};
use crate::bot::{branding, components};
use crate::domain::economy::format_bac;
use crate::domain::resources::find_item;
use crate::infrastructure::repositories::giveaway_repo::Giveaway;
use serenity::all::*;

pub fn giveaway_active_embed(gw: &Giveaway, entries: i64) -> CreateEmbed {
    let ts = gw.ends_at.timestamp();
    let (color, header, desc) = match gw.kind.as_str() {
        KIND_CASH => (
            branding::COLOR_ECONOMY,
            "💰 Urbaniac Cash Giveaway",
            "Press **Join Giveaway** below! Requires an Urbaniac profile (**Play** or **Urbaniac Activity**). Cash is credited automatically when the giveaway ends.",
        ),
        KIND_ITEM => (
            branding::COLOR_INVENTORY,
            "🎁 Urbaniac Item Giveaway",
            "Press **Join Giveaway** below! Requires an Urbaniac profile (**Play** or **Urbaniac Activity**). Items are delivered automatically to your inventory (or Pending Claim if full).",
        ),
        _ => (
            branding::COLOR_SOCIAL,
            "🎉 Community Giveaway",
            "Press **Join Giveaway** below to enter!",
        ),
    };

    branding::styled_embed(color)
        .title(header)
        .description(desc)
        .field("Prize", format!("**{}**", gw.prize), false)
        .field("Winners", gw.winner_count.to_string(), true)
        .field("Entries", entries.to_string(), true)
        .field("Ends", format!("<t:{ts}:R> (<t:{ts}:F>)"), false)
        .field("Hosted by", format!("<@{}>", gw.host_id), true)
        .footer(CreateEmbedFooter::new(format!(
            "Urbaniac • Giveaway #{}",
            gw.id
        )))
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

    let title = match gw.kind.as_str() {
        KIND_CASH => "💰 Cash Giveaway Ended",
        KIND_ITEM => "🎁 Item Giveaway Ended",
        _ => "🎉 Giveaway Ended",
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
            format!(
                "🔁 **Giveaway #{} Reroll!** New winner: {mentions} — won **{}**!",
                gw.id, gw.prize
            )
        } else {
            format!(
                "🎉 **Congratulations {mentions}!** You won **{}** (Giveaway #{})!",
                gw.prize, gw.id
            )
        };

        let mut parts = vec![prefix];
        match gw.kind.as_str() {
            KIND_CASH => {
                if !res.credited_winners.is_empty() {
                    parts.push(format!(
                        "💰 **{}** has been credited automatically to the winner's Urbaniac wallet.",
                        format_bac(gw.cash_amount)
                    ));
                }
            }
            KIND_ITEM => {
                let item_name = gw
                    .item_id
                    .as_deref()
                    .and_then(find_item)
                    .map(|i| i.name)
                    .unwrap_or("Item");
                if !res.credited_winners.is_empty() {
                    parts.push(format!(
                        "🎁 **{}x {}** has been delivered to the winner's Urbaniac inventory.",
                        gw.item_qty, item_name
                    ));
                }
                if !res.pending_winners.is_empty() {
                    let pending_mentions = res
                        .pending_winners
                        .iter()
                        .map(|id| format!("<@{id}>"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    parts.push(format!(
                        "📦 {pending_mentions}: your inventory is full — claim your item from **Pending Claim** inside **Inventory**."
                    ));
                }
            }
            _ => {}
        }

        if !res.skipped_no_profile.is_empty() {
            let missing = res
                .skipped_no_profile
                .iter()
                .map(|id| format!("<@{id}>"))
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!(
                "⚠️ Could not deliver reward to {missing} because they do not have an Urbaniac profile yet."
            ));
        }

        parts.join("\n")
    };

    let msg = CreateMessage::new().content(summary);
    if let Err(e) = channel.send_message(http, msg).await {
        tracing::warn!(giveaway_id = gw.id, error = ?e, "failed to send giveaway winner message");
    }
}
