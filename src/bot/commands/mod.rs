pub mod announce;
pub mod giveaway;
pub mod help;
pub mod host;
pub mod start;
pub mod welcome;

use crate::application::server::{MAX_DURATION_MINUTES, MAX_WINNERS, MIN_DURATION_MINUTES, MIN_WINNERS};
use serenity::all::{ChannelType, CommandOptionType, CreateCommand, CreateCommandOption, Permissions};

pub fn definitions() -> Vec<CreateCommand> {
    let text_channels = vec![ChannelType::Text, ChannelType::News];

    vec![
        CreateCommand::new("start").description("Start playing Urbaniac and open your profile"),
        CreateCommand::new("help").description("Short gameplay guide for Urbaniac"),
        CreateCommand::new("host")
            .description("Post the Play Urbaniac panel (creates a private thread per player)")
            .default_member_permissions(Permissions::MANAGE_CHANNELS),
        CreateCommand::new("welcome")
            .description("Configure welcome greet messages and auto-role")
            .default_member_permissions(Permissions::MANAGE_GUILD)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "setup",
                    "Set welcome channel, optional auto-role, and custom message",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Channel,
                        "channel",
                        "Channel where welcome messages will be sent",
                    )
                    .channel_types(text_channels.clone())
                    .required(true),
                )
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::Role,
                    "role",
                    "Optional role assigned automatically when a member joins",
                )),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "status",
                "View current welcome greet configuration",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "disable",
                "Disable welcome greet and auto-role",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "test",
                "Preview the welcome greet message privately",
            )),
        CreateCommand::new("announce")
            .description("Send an embed announcement to a channel")
            .default_member_permissions(Permissions::MANAGE_GUILD)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Channel,
                    "channel",
                    "Target channel for the announcement",
                )
                .channel_types(text_channels.clone())
                .required(true),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::Role,
                "mention",
                "Optional role to mention",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Boolean,
                "everyone",
                "Ping @everyone with this announcement",
            )),
        CreateCommand::new("giveaway")
            .description("Host or manage community giveaways")
            .default_member_permissions(Permissions::MANAGE_GUILD)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "start",
                    "Start a standard community giveaway",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "prize",
                        "Prize description (e.g. Discord Nitro, Custom Role)",
                    )
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
                ),
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
        CreateCommand::new("cash-giveaway")
            .description("Start an in-game Urbaniac Cash (Bac) giveaway")
            .default_member_permissions(Permissions::MANAGE_GUILD)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "amount",
                    "Cash (Bac) awarded to each winner",
                )
                .min_int_value(1)
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "duration_minutes",
                    "Duration in minutes (1 to 43200)",
                )
                .min_int_value(MIN_DURATION_MINUTES as u64)
                .max_int_value(MAX_DURATION_MINUTES as u64)
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "winners",
                    "Number of winners (default 1, max 20)",
                )
                .min_int_value(MIN_WINNERS as u64)
                .max_int_value(MAX_WINNERS as u64),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Channel,
                    "channel",
                    "Channel to post the giveaway in (defaults to current channel)",
                )
                .channel_types(text_channels),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "note",
                "Optional event note shown with the prize",
            )),
    ]
}
