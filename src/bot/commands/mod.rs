pub mod giveaway;
pub mod help;
pub mod setup;

use crate::application::giveaway::{
    MAX_CASH_AMOUNT, MAX_DURATION_MINUTES, MAX_ITEM_QTY, MAX_WINNERS, MIN_DURATION_MINUTES,
    MIN_WINNERS,
};
use crate::domain::resources::ITEMS;
use serenity::all::{
    ChannelType, CommandOptionType, CreateCommand, CreateCommandOption, InteractionContext,
    Permissions,
};

pub const NAMES: &[&str] = &["help", "setup", "giveaway"];

pub fn definitions() -> Vec<CreateCommand> {
    let text_channels = vec![ChannelType::Text, ChannelType::News];

    let mut item_option = CreateCommandOption::new(
        CommandOptionType::String,
        "item",
        "In-game item awarded to each winner",
    )
    .required(true);
    for def in ITEMS {
        item_option = item_option.add_string_choice(format!("{} ({})", def.name, def.rarity), def.id);
    }

    vec![
        CreateCommand::new("help")
            .description("Short gameplay guide for Urbaniac")
            .contexts(vec![InteractionContext::Guild]),
        CreateCommand::new("setup")
            .description("Post the Play Urbaniac panel to a target channel (Bot Owner only)")
            .default_member_permissions(Permissions::MANAGE_CHANNELS)
            .contexts(vec![InteractionContext::Guild])
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Channel,
                    "channel",
                    "Target channel to post the panel (defaults to current channel)",
                )
                .channel_types(text_channels.clone())
                .required(false),
            ),
        CreateCommand::new("giveaway")
            .description("Host or manage in-game Urbaniac giveaways (Admin only)")
            .default_member_permissions(Permissions::MANAGE_GUILD)
            .contexts(vec![InteractionContext::Guild])
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "cash",
                    "Start an in-game Bac (cash) giveaway",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "amount",
                        "Cash (Bac) awarded to each winner",
                    )
                    .min_int_value(1)
                    .max_int_value(MAX_CASH_AMOUNT as u64)
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "duration_minutes",
                        "Duration in minutes (1 to 43200)",
                    )
                    .min_int_value(MIN_DURATION_MINUTES as u64)
                    .max_int_value(MAX_DURATION_MINUTES as u64)
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "winners",
                        "Number of winners (default 1, max 20)",
                    )
                    .min_int_value(MIN_WINNERS as u64)
                    .max_int_value(MAX_WINNERS as u64),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Channel,
                        "channel",
                        "Channel to post the giveaway in (defaults to current channel)",
                    )
                    .channel_types(text_channels.clone()),
                )
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::String,
                    "note",
                    "Optional event note shown with the prize",
                )),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "item",
                    "Start an in-game item giveaway",
                )
                .add_sub_option(item_option)
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "quantity",
                        "Item quantity awarded to each winner",
                    )
                    .min_int_value(1)
                    .max_int_value(MAX_ITEM_QTY as u64)
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "duration_minutes",
                        "Duration in minutes (1 to 43200)",
                    )
                    .min_int_value(MIN_DURATION_MINUTES as u64)
                    .max_int_value(MAX_DURATION_MINUTES as u64)
                    .required(true),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "winners",
                        "Number of winners (default 1, max 20)",
                    )
                    .min_int_value(MIN_WINNERS as u64)
                    .max_int_value(MAX_WINNERS as u64),
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Channel,
                        "channel",
                        "Channel to post the giveaway in (defaults to current channel)",
                    )
                    .channel_types(text_channels),
                )
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::String,
                    "note",
                    "Optional event note shown with the prize",
                )),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "end",
                    "End an active giveaway immediately",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "id",
                        "Giveaway ID",
                    )
                    .min_int_value(1)
                    .required(true),
                ),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "reroll",
                    "Pick a new winner for an ended giveaway",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "id",
                        "Giveaway ID",
                    )
                    .min_int_value(1)
                    .required(true),
                ),
            ),
    ]
}
