use rust_decimal::Decimal;

pub const MARKET_TICK_SECS: i64 = 900; // 15 menit (diskalakan ACTIVITY_DURATION_PCT)
pub const MAX_ACTIVE_LISTINGS: i64 = 5;
pub const LISTING_FEE_PCT: i64 = 2;
pub const MIN_LISTING_FEE_CENTS: i64 = 100; // 1.00 Bac
pub const BROKER_COMMISSION_PCT: i64 = 5;

/// Batas multiplier harga dinamis NPC (0.70x .. 1.50x harga dasar).
pub fn min_multiplier() -> Decimal {
    Decimal::new(7000, 4)
}

pub fn max_multiplier() -> Decimal {
    Decimal::new(15000, 4)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListingDuration {
    Short,  // 1 jam
    Medium, // 6 jam
    Long,   // 24 jam
}

impl ListingDuration {
    pub fn as_str(&self) -> &'static str {
        match self {
            ListingDuration::Short => "1h",
            ListingDuration::Medium => "6h",
            ListingDuration::Long => "24h",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "1h" => Some(ListingDuration::Short),
            "6h" => Some(ListingDuration::Medium),
            "24h" => Some(ListingDuration::Long),
            _ => None,
        }
    }

    pub fn secs(&self) -> i64 {
        match self {
            ListingDuration::Short => 3600,
            ListingDuration::Medium => 21_600,
            ListingDuration::Long => 86_400,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ListingDuration::Short => "1 Jam",
            ListingDuration::Medium => "6 Jam",
            ListingDuration::Long => "24 Jam",
        }
    }
}

/// Menghitung harga pasar dinamis dari harga dasar NPC dan multiplier.
pub fn dynamic_price(base_price: Decimal, multiplier: Decimal) -> Decimal {
    (base_price * multiplier)
        .round_dp(2)
        .max(Decimal::new(1, 2))
}

/// Menghitung multiplier berikutnya berdasarkan mean-reversion, permintaan NPC acak (`demand_bps`),
/// dan tekanan suplai dari penjualan pemain (`recent_sales`).
pub fn next_multiplier(current: Decimal, recent_sales: i64, demand_bps: i64) -> Decimal {
    let one = Decimal::ONE;
    // Mean reversion ringan (15% kembali ke 1.00)
    let reverted = current + (one - current) * Decimal::new(15, 2);
    // Perubahan permintaan NPC (-800..=+800 bps => -0.08..=+0.08)
    let demand_shift = Decimal::new(demand_bps.clamp(-800, 800), 4);
    // Tekanan penjualan: maks -0.10 (-10%) untuk >= 200 unit terjual
    let supply_bps = (recent_sales.max(0).min(200)) * 5;
    let supply_shift = Decimal::new(supply_bps, 4);

    (reverted + demand_shift - supply_shift)
        .round_dp(4)
        .clamp(min_multiplier(), max_multiplier())
}

/// Fee listing dibayar di muka: 2% dari total nilai target, minimal 1.00 Bac.
pub fn calc_listing_fee(target_unit_price: Decimal, qty: i64) -> Decimal {
    let total = target_unit_price * Decimal::from(qty.max(0));
    let pct = (total * Decimal::new(LISTING_FEE_PCT, 2)).round_dp(2);
    pct.max(Decimal::new(MIN_LISTING_FEE_CENTS, 2))
}

/// Hasil kotor, komisi broker (5%), dan hasil bersih ke Wallet saat terjual.
pub fn calc_sale_settlement(unit_price: Decimal, qty: i64) -> (Decimal, Decimal, Decimal) {
    let gross = (unit_price * Decimal::from(qty.max(0))).round_dp(2);
    let commission = (gross * Decimal::new(BROKER_COMMISSION_PCT, 2)).round_dp(2);
    let net = (gross - commission).max(Decimal::ZERO);
    (gross, commission, net)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_price_rounds_and_has_floor() {
        assert_eq!(dynamic_price(Decimal::new(300, 2), Decimal::new(12500, 4)), Decimal::new(375, 2));
        assert_eq!(dynamic_price(Decimal::new(1, 2), Decimal::new(7000, 4)), Decimal::new(1, 2));
    }

    #[test]
    fn next_multiplier_stays_within_bounds() {
        let high = next_multiplier(max_multiplier(), 0, 800);
        assert!(high <= max_multiplier());
        let low = next_multiplier(min_multiplier(), 500, -800);
        assert!(low >= min_multiplier());
    }

    #[test]
    fn listing_fee_and_commission_math() {
        assert_eq!(calc_listing_fee(Decimal::new(300, 2), 1), Decimal::new(100, 2)); // min 1.00
        assert_eq!(calc_listing_fee(Decimal::new(1000, 2), 10), Decimal::new(200, 2)); // 2% of 100 = 2.00

        let (gross, comm, net) = calc_sale_settlement(Decimal::new(1000, 2), 10);
        assert_eq!(gross, Decimal::new(10000, 2));
        assert_eq!(comm, Decimal::new(500, 2));
        assert_eq!(net, Decimal::new(9500, 2));
    }
}
