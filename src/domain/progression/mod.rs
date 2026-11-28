use crate::domain::city::CityStage;
use crate::domain::player::xp_to_next;

pub const NO_CAP: i32 = i32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XpState {
    pub level: i32,
    /// XP di dalam level saat ini.
    pub xp: i64,
    /// XP yang tertahan karena level sudah di cap.
    pub overflow: i64,
}

impl XpState {
    pub const START: XpState = XpState { level: 1, xp: 0, overflow: 0 };
}

/// Level cap aktivitas mengikuti City Stage. Nilai awal sementara.
pub fn activity_level_cap(stage: CityStage) -> i32 {
    match stage {
        CityStage::Kampung => 10,
        CityStage::Desa => 20,
        CityStage::KotaKecil => 30,
        CityStage::KotaDagang => 40,
        CityStage::Metropolis => 50,
    }
}

/// Menambah XP. Overflow lama ikut diterapkan jika cap sudah naik.
/// XP berlebih saat di cap disimpan sebagai overflow.
pub fn apply_xp(mut s: XpState, gain: i64, cap: i32) -> XpState {
    let mut pool = gain.max(0) + s.overflow;
    s.overflow = 0;
    loop {
        if s.level >= cap {
            s.overflow = pool;
            break;
        }
        let need = xp_to_next(s.level) - s.xp;
        if pool >= need {
            pool -= need;
            s.level += 1;
            s.xp = 0;
        } else {
            s.xp += pool;
            break;
        }
    }
    s
}

#[derive(Debug)]
pub struct ProgressionReward {
    pub key: &'static str,
    pub min_player_level: i32,
    pub bac_cents: i64,
    pub title: &'static str,
}

pub const PROGRESSION_REWARDS: &[ProgressionReward] = &[
    ProgressionReward {
        key: "player_lv_5",
        min_player_level: 5,
        bac_cents: 10_000,
        title: "Model Citizen (Lv 5)",
    },
    ProgressionReward {
        key: "player_lv_10",
        min_player_level: 10,
        bac_cents: 25_000,
        title: "Town Mover (Lv 10)",
    },
    ProgressionReward {
        key: "player_lv_15",
        min_player_level: 15,
        bac_cents: 40_000,
        title: "Trade Notable (Lv 15)",
    },
    ProgressionReward {
        key: "player_lv_25",
        min_player_level: 25,
        bac_cents: 80_000,
        title: "Metropolis Architect (Lv 25)",
    },
];

pub fn eligible_rewards(player_level: i32) -> impl Iterator<Item = &'static ProgressionReward> {
    PROGRESSION_REWARDS
        .iter()
        .filter(move |r| player_level >= r.min_player_level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_up_and_carries_remainder() {
        let s = apply_xp(XpState::START, 150, 10); // L1 butuh 100
        assert_eq!((s.level, s.xp, s.overflow), (2, 50, 0));
    }

    #[test]
    fn exact_threshold_levels_up_with_zero_xp() {
        let s = apply_xp(XpState::START, 100, 10);
        assert_eq!((s.level, s.xp), (2, 0));
    }

    #[test]
    fn excess_at_cap_becomes_overflow_then_applies_when_cap_rises() {
        let s = apply_xp(XpState { level: 9, xp: 0, overflow: 0 }, 1000, 10); // L9 butuh 900
        assert_eq!((s.level, s.xp, s.overflow), (10, 0, 100));
        let s = apply_xp(s, 0, 20);
        assert_eq!((s.level, s.xp, s.overflow), (10, 100, 0));
    }

    #[test]
    fn negative_gain_is_ignored() {
        let s = apply_xp(XpState::START, -5, 10);
        assert_eq!(s, XpState::START);
    }

    #[test]
    fn caps_increase_with_stage() {
        assert!(activity_level_cap(CityStage::Desa) > activity_level_cap(CityStage::Kampung));
    }

    #[test]
    fn overflow_cascades_across_multiple_stage_caps() {
        // Kampung cap = 10; L9 butuh 900, L10..19 butuh besar -> beri 5000 XP
        let s = apply_xp(XpState { level: 9, xp: 0, overflow: 0 }, 5_000, activity_level_cap(CityStage::Kampung));
        assert_eq!(s.level, 10);
        assert_eq!(s.overflow, 4_100);

        // Naik ke Desa (cap 20): overflow otomatis naikkan level
        let s2 = apply_xp(s, 0, activity_level_cap(CityStage::Desa));
        assert!(s2.level > 10 && s2.level <= 20);
        assert_eq!(s2.overflow, 0);

        // Reward milestone cek
        let keys: Vec<_> = eligible_rewards(10).map(|r| r.key).collect();
        assert_eq!(keys, vec!["player_lv_5", "player_lv_10"]);
    }
}
