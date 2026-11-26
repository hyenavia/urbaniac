pub mod help;
pub mod start;

use serenity::all::CreateCommand;

pub fn definitions() -> Vec<CreateCommand> {
    vec![
        CreateCommand::new("start").description("Mulai bermain Urbaniac dan buat profilmu"),
        CreateCommand::new("help").description("Panduan singkat Urbaniac"),
    ]
}
