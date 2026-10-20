// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

use drivich_core::config::Config;
use drivich_core::db::Pool;
use drivich_core::storage::Storage;
use drivich_core::uploads::Limits;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool,
    pub storage: Storage,
    pub cfg: Arc<Config>,
    pub limits: Limits,
    pub limiter: Arc<Limiter>,
    /// Caps simultaneous .zip downloads.
    pub zip_slots: Arc<tokio::sync::Semaphore>,
}

/// Tiny in-memory throttle for failed logins: `max` failures per `window`
/// per key (email + client address), then 429 until the window passes.
pub struct Limiter {
    inner: Mutex<HashMap<String, (u32, Instant)>>,
    max: u32,
    window: Duration,
}

impl Limiter {
    pub fn new(max: u32, window: Duration) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            max,
            window,
        }
    }

    pub fn blocked(&self, key: &str) -> bool {
        let mut m = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match m.get(key) {
            Some((n, since)) if since.elapsed() < self.window => *n >= self.max,
            Some(_) => {
                m.remove(key);
                false
            }
            None => false,
        }
    }

    pub fn record_failure(&self, key: &str) {
        let mut m = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if m.len() > 10_000 {
            let window = self.window;
            m.retain(|_, (_, since)| since.elapsed() < window);
        }
        let e = m.entry(key.to_string()).or_insert((0, Instant::now()));
        if e.1.elapsed() >= self.window {
            *e = (0, Instant::now());
        }
        e.0 += 1;
    }

    pub fn reset(&self, key: &str) {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_after_max_failures_and_resets() {
        let l = Limiter::new(3, Duration::from_secs(60));
        assert!(!l.blocked("a"));
        for _ in 0..3 {
            l.record_failure("a");
        }
        assert!(l.blocked("a"));
        assert!(!l.blocked("b"));
        l.reset("a");
        assert!(!l.blocked("a"));
    }
}
