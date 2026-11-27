pub mod help;
pub mod host;
pub mod start;

use serenity::all::{CreateCommand, Permissions};

pub fn definitions() -> Vec<CreateCommand> {
    vec![
        CreateCommand::new("start").description("Mulai bermain Urbaniac dan buat profilmu"),
        CreateCommand::new("help").description("Panduan singkat Urbaniac"),
        CreateCommand::new("host")
            .description("Tampilkan panel Play Urbaniac (membuat private thread per pemain)")
            .default_member_permissions(Permissions::MANAGE_CHANNELS),
    ]
}
