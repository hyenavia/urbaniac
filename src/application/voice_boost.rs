use std::collections::HashSet;
use std::sync::RwLock;

pub const VOICE_BOOST_SPEED_PCT: i64 = 20;
pub const VOICE_BOOST_XP_PCT: i64 = 15;

pub fn apply_xp_bonus(base_xp: i64, bonus_pct: i64) -> i64 {
    if bonus_pct <= 0 || base_xp <= 0 {
        return base_xp;
    }
    ((base_xp * (100 + bonus_pct)) + 50) / 100
}

#[derive(Debug, Default)]
pub struct VoiceBoostTracker {
    guild_id: Option<u64>,
    channel_id: Option<u64>,
    active_users: RwLock<HashSet<i64>>,
}

impl VoiceBoostTracker {
    pub fn new(guild_id: Option<u64>, channel_id: Option<u64>) -> Self {
        Self {
            guild_id,
            channel_id,
            active_users: RwLock::new(HashSet::new()),
        }
    }

    pub fn disabled() -> Self {
        Self::new(None, None)
    }

    pub fn is_boosted(&self, player_id: i64) -> bool {
        if self.channel_id.is_none() {
            return false;
        }
        self.active_users
            .read()
            .map(|set| set.contains(&player_id))
            .unwrap_or(false)
    }

    pub fn speed_bonus_pct(&self, player_id: i64) -> i64 {
        if self.is_boosted(player_id) {
            VOICE_BOOST_SPEED_PCT
        } else {
            0
        }
    }

    pub fn xp_bonus_pct(&self, player_id: i64) -> i64 {
        if self.is_boosted(player_id) {
            VOICE_BOOST_XP_PCT
        } else {
            0
        }
    }

    pub fn sync_guild_voice_states<I>(&self, guild_id: u64, states: I)
    where
        I: IntoIterator<Item = (u64, Option<u64>)>,
    {
        let Some(target_channel) = self.channel_id else {
            return;
        };
        if let Some(allowed_guild) = self.guild_id {
            if guild_id != allowed_guild {
                return;
            }
        }
        if let Ok(mut set) = self.active_users.write() {
            set.clear();
            for (uid, ch) in states {
                if ch == Some(target_channel) {
                    set.insert(uid as i64);
                }
            }
        }
    }

    /// Updates membership on a Discord VoiceStateUpdate.
    /// Returns `Some(true)` if the player gained boost, `Some(false)` if they lost it, `None` if unchanged.
    pub fn handle_voice_update(
        &self,
        guild_id: Option<u64>,
        user_id: u64,
        new_channel_id: Option<u64>,
    ) -> Option<bool> {
        let target_channel = self.channel_id?;
        if let Some(allowed_guild) = self.guild_id {
            if guild_id != Some(allowed_guild) {
                return None;
            }
        }
        let pid = user_id as i64;
        let should_boost = new_channel_id == Some(target_channel);
        let mut set = self.active_users.write().ok()?;
        if should_boost {
            if set.insert(pid) {
                Some(true)
            } else {
                None
            }
        } else if set.remove(&pid) {
            Some(false)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_xp_bonus_with_rounding() {
        assert_eq!(apply_xp_bonus(35, 0), 35);
        assert_eq!(apply_xp_bonus(35, 15), 40); // 35 * 1.15 = 40.25 -> 40
        assert_eq!(apply_xp_bonus(40, 15), 46); // 40 * 1.15 = 46
        assert_eq!(apply_xp_bonus(25, 15), 29); // 25 * 1.15 = 28.75 -> 29
        assert_eq!(apply_xp_bonus(0, 15), 0);
    }

    #[test]
    fn tracks_join_switch_leave_and_guild_sync() {
        let tracker = VoiceBoostTracker::new(Some(10), Some(200));
        assert!(!tracker.is_boosted(1));
        assert_eq!(tracker.speed_bonus_pct(1), 0);
        assert_eq!(tracker.xp_bonus_pct(1), 0);

        // Wrong guild is ignored
        assert_eq!(tracker.handle_voice_update(Some(99), 1, Some(200)), None);
        assert!(!tracker.is_boosted(1));

        // Join target channel
        assert_eq!(tracker.handle_voice_update(Some(10), 1, Some(200)), Some(true));
        assert!(tracker.is_boosted(1));
        assert_eq!(tracker.speed_bonus_pct(1), VOICE_BOOST_SPEED_PCT);
        assert_eq!(tracker.xp_bonus_pct(1), VOICE_BOOST_XP_PCT);

        // Mute/unmute in same channel returns None (no state change)
        assert_eq!(tracker.handle_voice_update(Some(10), 1, Some(200)), None);
        assert!(tracker.is_boosted(1));

        // Move to another voice channel removes boost
        assert_eq!(tracker.handle_voice_update(Some(10), 1, Some(201)), Some(false));
        assert!(!tracker.is_boosted(1));

        // Re-join and disconnect
        assert_eq!(tracker.handle_voice_update(Some(10), 1, Some(200)), Some(true));
        assert_eq!(tracker.handle_voice_update(Some(10), 1, None), Some(false));
        assert!(!tracker.is_boosted(1));

        // Guild sync replaces set accurately
        tracker.sync_guild_voice_states(
            10,
            [(2, Some(200)), (3, Some(201)), (4, None), (5, Some(200))],
        );
        assert!(!tracker.is_boosted(1));
        assert!(tracker.is_boosted(2));
        assert!(!tracker.is_boosted(3));
        assert!(!tracker.is_boosted(4));
        assert!(tracker.is_boosted(5));
    }
}
