use std::prelude::v1::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use log::info;

use crate::pipeline::StepInfo;

use super::{Hook, NodeControl};

pub struct CacheHook {
    cache_dir: PathBuf,
    node_keys: Mutex<HashMap<usize, u64>>,
}

impl Default for CacheHook {
    fn default() -> Self {
        Self::new(".pondcache")
    }
}

impl CacheHook {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            node_keys: Mutex::new(HashMap::new()),
        }
    }

    fn compute_cache_key(&self, n: &dyn StepInfo) -> Option<u64> {
        use core::hash::{Hash, Hasher};
        let mut hasher = std::hash::DefaultHasher::new();
        n.name().hash(&mut hasher);
        n.type_string().hash(&mut hasher);

        let node_keys = self.node_keys.lock().unwrap();
        let mut input_count = 0u32;
        let mut failed = false;
        n.for_each_input(&mut |ds| {
            if failed { return; }
            input_count += 1;
            ds.meta.type_string().hash(&mut hasher);
            if let Some(key) = node_keys.get(&ds.id) {
                key.hash(&mut hasher);
            } else if let Some(h) = ds.meta.content_hash() {
                h.hash(&mut hasher);
            } else {
                failed = true;
            }
        });
        if failed { return None; }
        input_count.hash(&mut hasher);

        n.for_each_output(&mut |ds| {
            ds.meta.type_string().hash(&mut hasher);
        });

        Some(hasher.finish())
    }

    fn outputs_are_persistent(n: &dyn StepInfo) -> bool {
        let mut all_persistent = true;
        let mut has_outputs = false;
        n.for_each_output(&mut |ds| {
            has_outputs = true;
            if !ds.meta.is_persistent() {
                all_persistent = false;
            }
        });
        has_outputs && all_persistent
    }

    fn record_node_key(&self, n: &dyn StepInfo, key: u64) {
        let mut node_keys = self.node_keys.lock().unwrap();
        n.for_each_output(&mut |ds| {
            node_keys.insert(ds.id, key);
        });
    }

    fn cache_path(&self, node_name: &str) -> PathBuf {
        let sanitized: String = node_name.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        self.cache_dir.join(sanitized)
    }

    fn read_cached_key(&self, node_name: &str) -> Option<u64> {
        let path = self.cache_path(node_name);
        let content = std::fs::read_to_string(path).ok()?;
        content.trim().parse().ok()
    }

    fn write_cached_key(&self, node_name: &str, key: u64) {
        let path = self.cache_path(node_name);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, key.to_string());
    }
}

impl Hook for CacheHook {
    fn node_control(&self, n: &dyn StepInfo) -> NodeControl {
        if !Self::outputs_are_persistent(n) {
            return NodeControl::Run;
        }
        let key = match self.compute_cache_key(n) {
            Some(k) => k,
            None => return NodeControl::Run,
        };
        match self.read_cached_key(n.name()) {
            Some(cached) if cached == key => NodeControl::Skip,
            _ => NodeControl::Run,
        }
    }

    fn after_node_run(&self, n: &dyn StepInfo, skipped: bool) {
        if let Some(key) = self.compute_cache_key(n) {
            if skipped {
                info!("[cache] {} - skipped (inputs unchanged)", n.name());
            } else {
                self.write_cached_key(n.name(), key);
            }
            self.record_node_key(n, key);
        }
    }
}
