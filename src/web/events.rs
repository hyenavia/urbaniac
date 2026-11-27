use crate::web::error::ApiError;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

pub const MAX_SSE_TOTAL: usize = 64;
pub const MAX_SSE_PER_USER: usize = 3;
const BROADCAST_CAPACITY: usize = 128;

#[derive(Debug, Clone, Serialize)]
pub struct RealtimeEvent {
    /// `Some(uid)` hanya dikirim ke pemain `uid`; `None` dikirim ke semua klien aktif.
    #[serde(skip_serializing)]
    pub player_id: Option<i64>,
    pub kind: &'static str,
    pub message: Option<String>,
}

#[derive(Default)]
struct SseTracker {
    total: usize,
    per_user: HashMap<i64, usize>,
}

pub struct SseGuard {
    user_id: i64,
    tracker: Arc<Mutex<SseTracker>>,
}

impl Drop for SseGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.tracker.lock() {
            guard.total = guard.total.saturating_sub(1);
            if let Some(cnt) = guard.per_user.get_mut(&self.user_id) {
                *cnt = cnt.saturating_sub(1);
                if *cnt == 0 {
                    guard.per_user.remove(&self.user_id);
                }
            }
        }
    }
}

pub struct EventHub {
    tx: broadcast::Sender<RealtimeEvent>,
    tracker: Arc<Mutex<SseTracker>>,
    max_total: usize,
    max_per_user: usize,
}

impl Default for EventHub {
    fn default() -> Self {
        Self::new(MAX_SSE_TOTAL, MAX_SSE_PER_USER)
    }
}

impl EventHub {
    pub fn new(max_total: usize, max_per_user: usize) -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            tx,
            tracker: Arc::new(Mutex::new(SseTracker::default())),
            max_total: max_total.max(1),
            max_per_user: max_per_user.max(1),
        }
    }

    pub fn subscribe(
        &self,
        user_id: i64,
    ) -> Result<(broadcast::Receiver<RealtimeEvent>, SseGuard), ApiError> {
        let mut tracker = self.tracker.lock().expect("sse tracker mutex poisoned");
        if tracker.total >= self.max_total {
            return Err(ApiError::too_many_requests(
                "sse_limit_reached",
                "Too many active real-time connections",
            ));
        }
        let user_count = tracker.per_user.get(&user_id).copied().unwrap_or(0);
        if user_count >= self.max_per_user {
            return Err(ApiError::too_many_requests(
                "sse_user_limit_reached",
                "Too many open Activity tabs for this account",
            ));
        }
        tracker.total += 1;
        tracker.per_user.insert(user_id, user_count + 1);

        let rx = self.tx.subscribe();
        let guard = SseGuard {
            user_id,
            tracker: self.tracker.clone(),
        };
        Ok((rx, guard))
    }

    pub fn publish(&self, event: RealtimeEvent) {
        let _ = self.tx.send(event);
    }

    pub fn is_visible_to(event: &RealtimeEvent, user_id: i64) -> bool {
        event.player_id.is_none() || event.player_id == Some(user_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_per_user_and_total_limits_and_releases_on_drop() {
        let hub = EventHub::new(3, 2);

        let (_rx1, g1) = hub.subscribe(1001).unwrap();
        let (_rx2, g2) = hub.subscribe(1001).unwrap();
        assert!(hub.subscribe(1001).is_err());

        let (_rx3, g3) = hub.subscribe(1002).unwrap();
        assert!(hub.subscribe(1003).is_err());

        drop(g1);
        let (_rx4, _g4) = hub.subscribe(1001).unwrap();
        drop(g2);
        drop(g3);
    }

    #[test]
    fn visibility_isolates_player_events() {
        let private_ev = RealtimeEvent {
            player_id: Some(1001),
            kind: "activity_complete",
            message: Some("done".to_string()),
        };
        assert!(EventHub::is_visible_to(&private_ev, 1001));
        assert!(!EventHub::is_visible_to(&private_ev, 1002));

        let global_ev = RealtimeEvent {
            player_id: None,
            kind: "state_sync",
            message: None,
        };
        assert!(EventHub::is_visible_to(&global_ev, 1001));
        assert!(EventHub::is_visible_to(&global_ev, 1002));
    }
}
