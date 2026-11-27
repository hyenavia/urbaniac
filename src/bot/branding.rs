use serenity::all::{Colour, CreateAttachment, CreateEmbed, CreateEmbedFooter, Timestamp};

pub const NAME: &str = "Urbaniac";
pub const STUDIO: &str = "Urbaniac Studio";
pub const TAGLINE: &str = "From Village to Metropolis.";
pub const SECONDARY_TAGLINE: &str = "Build Your Life. Shape Your City.";

pub const BOT_STATUS: &str = "🌱 Growing the city";
pub const FOOTER: &str = "Urbaniac • From Village to Metropolis.";

pub const COLOR_GENERAL: u32 = 0x79B85C;
pub const COLOR_EXPLORATION: u32 = 0x55BCE8;
pub const COLOR_ECONOMY: u32 = 0xFFD166;
pub const COLOR_ACTIVITIES: u32 = 0xFF914D;
pub const COLOR_SOCIAL: u32 = 0xF178A8;
pub const COLOR_INFRASTRUCTURE: u32 = 0x87939C;
pub const COLOR_INVENTORY: u32 = 0xA87955;
pub const COLOR_SYSTEM: u32 = 0x28354A;

pub const BANNER_FILENAME: &str = "banner.jpg";
const BANNER_BYTES: &[u8] = include_bytes!("../../assets/banner.jpg");

/// Creates a base embed with Urbaniac's brand color, footer, and timestamp.
pub fn styled_embed(color: u32) -> CreateEmbed {
    CreateEmbed::new()
        .colour(Colour::new(color))
        .footer(CreateEmbedFooter::new(FOOTER))
        .timestamp(Timestamp::now())
}

/// Returns the Urbaniac banner image as a Discord attachment.
pub fn banner_attachment() -> CreateAttachment {
    CreateAttachment::bytes(BANNER_BYTES, BANNER_FILENAME)
}

pub fn banner_url() -> String {
    format!("attachment://{BANNER_FILENAME}")
}
