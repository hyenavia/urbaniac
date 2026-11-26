use rust_decimal::Decimal;

/// Bac dengan 2 desimal. Nilai awal (bisa diubah, lihat GAME_RULES.md).
pub fn starter_bac() -> Decimal {
    Decimal::new(10_000, 2) // 100.00
}

pub fn format_bac(amount: Decimal) -> String {
    format!("{} Bac", amount.round_dp(2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_two_decimals() {
        assert_eq!(format_bac(Decimal::new(10_000, 2)), "100.00 Bac");
    }
}
