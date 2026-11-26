use serenity::all::*;

pub const PREFIX: &str = "ub";

pub fn main_menu(owner: u64) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{PREFIX}:profile:{owner}"))
            .label("Profil")
            .style(ButtonStyle::Primary),
        CreateButton::new(format!("{PREFIX}:inventory:{owner}"))
            .label("Inventory")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{PREFIX}:claim:{owner}"))
            .label("Klaim Pending")
            .style(ButtonStyle::Success),
    ])]
}

/// Format custom_id: `ub:<aksi>:<owner_discord_id>`.
pub fn parse_custom_id(id: &str) -> Option<(&str, u64)> {
    let mut parts = id.splitn(3, ':');
    if parts.next()? != PREFIX {
        return None;
    }
    let action = parts.next()?;
    let owner = parts.next()?.parse::<u64>().ok()?;
    Some((action, owner))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_id() {
        assert_eq!(parse_custom_id("ub:profile:123"), Some(("profile", 123)));
    }

    #[test]
    fn rejects_invalid_ids() {
        assert_eq!(parse_custom_id("x:profile:123"), None);
        assert_eq!(parse_custom_id("ub:profile"), None);
        assert_eq!(parse_custom_id("ub:profile:abc"), None);
    }
}
