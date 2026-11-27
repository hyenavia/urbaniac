use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MAX_RATE_LIMIT_KEYS: usize = 2048;

#[derive(Debug, Clone, Copy)]
struct Window {
    started_at: Instant,
    count: u32,
}

/// Fixed-window in-memory rate limiter with bounded key capacity.
pub struct RateLimiter {
    entries: Mutex<HashMap<String, Window>>,
    max_keys: usize,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(MAX_RATE_LIMIT_KEYS)
    }
}

impl RateLimiter {
    pub fn new(max_keys: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            max_keys: max_keys.max(1),
        }
    }

    pub fn check(&self, key: &str, max_requests: u32, window: Duration) -> bool {
        self.check_at(key, max_requests, window, Instant::now())
    }

    fn check_at(&self, key: &str, max_requests: u32, window: Duration, now: Instant) -> bool {
        let mut map = self.entries.lock().expect("rate limiter mutex poisoned");

        if let Some(entry) = map.get_mut(key) {
            if now.duration_since(entry.started_at) >= window {
                entry.started_at = now;
                entry.count = 1;
                return true;
            }
            if entry.count >= max_requests {
                return false;
            }
            entry.count += 1;
            return true;
        }

        if map.len() >= self.max_keys {
            map.retain(|_, v| now.duration_since(v.started_at) < window);
            if map.len() >= self.max_keys {
                if let Some(oldest_key) = map
                    .iter()
                    .min_by_key(|(_, v)| v.started_at)
                    .map(|(k, _)| k.clone())
                {
                    map.remove(&oldest_key);
                }
            }
        }

        map.insert(key.to_string(), Window { started_at: now, count: 1 });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_limit_and_resets_after_window() {
        let limiter = RateLimiter::new(16);
        let t0 = Instant::now();
        let win = Duration::from_secs(10);

        assert!(limiter.check_at("api:1", 2, win, t0));
        assert!(limiter.check_at("api:1", 2, win, t0 + Duration::from_secs(1)));
        assert!(!limiter.check_at("api:1", 2, win, t0 + Duration::from_secs(2)));

        assert!(limiter.check_at("api:1", 2, win, t0 + Duration::from_secs(10)));
    }

    #[test]
    fn stays_within_max_keys() {
        let limiter = RateLimiter::new(2);
        let t0 = Instant::now();
        let win = Duration::from_secs(60);

        assert!(limiter.check_at("k1", 5, win, t0));
        assert!(limiter.check_at("k2", 5, win, t0 + Duration::from_secs(1)));
        assert!(limiter.check_at("k3", 5, win, t0 + Duration::from_secs(2)));

        let map = limiter.entries.lock().unwrap();
        assert_eq!(map.len(), 2);
        assert!(!map.contains_key("k1"));
    }
}
