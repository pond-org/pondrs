//! Shared timing tracker for hooks.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

pub(crate) struct TimingTracker<K: Eq + std::hash::Hash> {
    timings: Mutex<HashMap<K, Instant>>,
}

impl<K: Eq + std::hash::Hash> TimingTracker<K> {
    pub fn new() -> Self {
        Self {
            timings: Mutex::new(HashMap::new()),
        }
    }

    pub fn start(&self, key: K) {
        self.timings.lock().unwrap().insert(key, Instant::now());
    }

    pub fn elapsed_ms(&self, key: &K) -> Option<f64> {
        self.timings
            .lock()
            .unwrap()
            .remove(key)
            .map(|start| start.elapsed().as_secs_f64() * 1000.0)
    }
}
