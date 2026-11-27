pub mod help;
pub mod host;
pub mod start;

use serenity::all::{CreateCommand, Permissions};

pub fn definitions() -> Vec<CreateCommand> {
    vec![
        CreateCommand::new("start").description("Start playing Urbaniac and open your profile"),
        CreateCommand::new("help").description("Short gameplay guide for Urbaniac"),
        CreateCommand::new("host")
            .description("Post the Play Urbaniac panel (creates a private thread per player)")
            .default_member_permissions(Permissions::MANAGE_CHANNELS),
    ]
}
