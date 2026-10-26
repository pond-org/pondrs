//! Cache hook: skips nodes whose inputs and configuration are unchanged.

use std::prelude::v1::*;
use core::fmt::Write as _;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use log::info;

use crate::pipeline::StepMeta;

use super::{Hook, HookAbort, HookControl};

/// Environment variable that, set to `off`, `0` or `false`, makes every
/// `CacheHook` behave as if built with [`CacheHook::force`]`(true)`.
pub const CACHE_ENV_VAR: &str = "PONDRS_CACHE";

/// Skips a node when its cache key matches the one stored by the last run
/// that ran it.
///
/// The key hashes the node's path and type, each input's and output's type
/// and configuration (`DatasetMeta::yaml`), and for each input either the
/// key of the node that produced it in this run — also when that node was
/// skipped — or its `content_hash()`. Only nodes whose outputs are all
/// persistent are skipped.
///
/// Keys are stored at `{cache_dir}/{node path}/{output config hash}`, one
/// directory level per path segment: two catalogs that write a node's outputs
/// to different places keep separate keys, two that write the same place share
/// one. Delete a directory (`rm -r .pondcache/epochs/3`) to invalidate the
/// nodes under it.
///
/// The cache trusts its records: a node whose key matches is skipped whether or
/// not its outputs still exist. If a node then has to run and one of its inputs,
/// produced by a skipped node, is gone (`content_hash()` is `None`, e.g. pruned
/// by `RetentionHook`), the run aborts naming the node and the producer.
///
/// Keys are hashed with `std`'s `DefaultHasher`, which is not guaranteed
/// stable across Rust versions; a change there makes every key miss once.
pub struct CacheHook {
    cache_dir: PathBuf,
    force: bool,
    /// Keys of the nodes seen in this run, by output dataset id.
    node_keys: Mutex<HashMap<usize, Recorded>>,
    /// Keys computed in `before_node_run`, by node path, for `after_node_run`.
    pending: Mutex<HashMap<String, Option<u64>>>,
}

struct Recorded {
    key: u64,
    /// Path of the producing node, if it was skipped rather than run.
    skipped_producer: Option<String>,
}

/// Why a node is not skipped, for the log.
enum RunReason {
    Forced,
    NotPersistent,
    NotHashable,
    NoStoredKey,
    KeyChanged,
}

impl RunReason {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Forced => "cache disabled",
            Self::NotPersistent => "outputs not persistent",
            Self::NotHashable => "an input has no content hash",
            Self::NoStoredKey => "no stored key",
            Self::KeyChanged => "key changed",
        }
    }
}

impl Default for CacheHook {
    fn default() -> Self {
        Self::new(".pondcache")
    }
}

impl CacheHook {
    /// A cache storing its keys under `cache_dir`.
    ///
    /// Reads [`CACHE_ENV_VAR`] once, here.
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            force: env_disables_cache(std::env::var(CACHE_ENV_VAR).ok().as_deref()),
            node_keys: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// With `true`, never skips a node, but still stores keys so that the
    /// next run can skip again. `false` keeps the [`CACHE_ENV_VAR`] setting.
    #[must_use]
    pub fn force(mut self, force: bool) -> Self {
        self.force |= force;
        self
    }

    fn compute_cache_key(&self, n: &dyn StepMeta) -> Option<u64> {
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
            ds.meta.yaml().hash(&mut hasher);
            if let Some(rec) = node_keys.get(&ds.id) {
                rec.key.hash(&mut hasher);
            } else if let Some(h) = ds.meta.content_hash() {
                h.hash(&mut hasher);
            } else {
                failed = true;
            }
        });
        if failed { return None; }
        input_count.hash(&mut hasher);

        Self::output_config_hash(n).hash(&mut hasher);
        Some(hasher.finish())
    }

    /// Hash of the outputs' types and configuration; names the key file.
    fn output_config_hash(n: &dyn StepMeta) -> u64 {
        use core::hash::{Hash, Hasher};
        let mut hasher = std::hash::DefaultHasher::new();
        n.for_each_output(&mut |ds| {
            ds.meta.type_string().hash(&mut hasher);
            ds.meta.yaml().hash(&mut hasher);
        });
        hasher.finish()
    }

    fn outputs_are_persistent(n: &dyn StepMeta) -> bool {
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

    fn record_node_key(&self, n: &dyn StepMeta, key: u64, skipped: bool) {
        let mut node_keys = self.node_keys.lock().unwrap();
        n.for_each_output(&mut |ds| {
            let skipped_producer = skipped.then(|| n.name().to_string());
            node_keys.insert(ds.id, Recorded { key, skipped_producer });
        });
    }

    /// Aborts if an input whose producer was skipped in this run is gone.
    fn check_skipped_inputs(&self, n: &dyn StepMeta) -> Result<(), HookAbort> {
        let node_keys = self.node_keys.lock().unwrap();
        let mut missing = None;
        n.for_each_input(&mut |ds| {
            if missing.is_some() { return; }
            let Some(producer) = node_keys.get(&ds.id).and_then(|r| r.skipped_producer.as_deref()) else {
                return;
            };
            if ds.meta.content_hash().is_none() {
                let config = ds.meta.yaml().map_or_else(String::new, |y| one_line(&y));
                missing = Some(format!(
                    "`{}` needs an input produced by skipped node `{producer}`, but it is missing \
                     (pruned?) [{config}]; delete its cache entry or rerun with {CACHE_ENV_VAR}=off",
                    n.name(),
                ));
            }
        });
        match missing {
            Some(msg) => {
                log::error!("[cache] {msg}");
                // `HookAbort` holds a `&'static str`; this leaks once per
                // aborting node, and the abort ends the run.
                Err(HookAbort(Box::leak(msg.into_boxed_str())))
            }
            None => Ok(()),
        }
    }

    fn node_dir(&self, node_path: &str) -> PathBuf {
        let mut dir = self.cache_dir.clone();
        for segment in node_path.split('/') {
            dir.push(escape_segment(segment));
        }
        dir
    }

    fn cache_path(&self, n: &dyn StepMeta) -> PathBuf {
        self.node_dir(n.name()).join(format!("{:016x}", Self::output_config_hash(n)))
    }

    fn read_cached_key(&self, n: &dyn StepMeta) -> Option<u64> {
        let content = std::fs::read_to_string(self.cache_path(n)).ok()?;
        content.lines().next()?.trim().parse().ok()
    }

    fn write_cached_key(&self, n: &dyn StepMeta, key: u64) {
        let path = self.cache_path(n);
        if let Some(parent) = path.parent() {
            self.create_node_dir(parent);
        }
        let mut content = format!("{key}\n# outputs:\n");
        n.for_each_output(&mut |ds| {
            if let Some(y) = ds.meta.yaml() {
                let _ = writeln!(content, "# - {}", one_line(&y));
            }
        });
        let _ = std::fs::write(path, content);
    }

    fn create_node_dir(&self, dir: &Path) {
        if std::fs::create_dir_all(dir).is_ok() {
            return;
        }
        // The earlier layout kept one file per node directly in the cache
        // directory; such a file can sit where a directory goes now.
        let Ok(rel) = dir.strip_prefix(&self.cache_dir) else { return };
        let mut p = self.cache_dir.clone();
        for c in rel.components() {
            p.push(c);
            if p.is_file() {
                let _ = std::fs::remove_file(&p);
            }
        }
        let _ = std::fs::create_dir_all(dir);
    }
}

impl Hook for CacheHook {
    fn before_node_run(&self, n: &dyn StepMeta) -> Result<HookControl, HookAbort> {
        let key = self.compute_cache_key(n);
        self.pending.lock().unwrap().insert(n.name().to_string(), key);

        let reason = if self.force {
            RunReason::Forced
        } else if !Self::outputs_are_persistent(n) {
            RunReason::NotPersistent
        } else {
            match key {
                None => RunReason::NotHashable,
                Some(key) => match self.read_cached_key(n) {
                    Some(cached) if cached == key => return Ok(HookControl::Skip),
                    Some(_) => RunReason::KeyChanged,
                    None => RunReason::NoStoredKey,
                },
            }
        };
        info!("[cache] {} - running ({})", n.name(), reason.as_str());
        self.check_skipped_inputs(n)?;
        Ok(HookControl::Continue)
    }

    fn after_node_run(&self, n: &dyn StepMeta, skipped: bool) -> Result<(), HookAbort> {
        let key = self.pending.lock().unwrap().remove(n.name()).flatten();
        if let Some(key) = key {
            if skipped {
                info!("[cache] {} - skipped (inputs unchanged)", n.name());
            } else if Self::outputs_are_persistent(n) {
                self.write_cached_key(n, key);
            }
            self.record_node_key(n, key, skipped);
        }
        Ok(())
    }
}

/// Whether a [`CACHE_ENV_VAR`] value turns the cache off.
fn env_disables_cache(value: Option<&str>) -> bool {
    value.is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "off" | "0" | "false"))
}

/// A path segment as a directory name: `.` / `..` and characters that are not
/// portable in file names are percent-encoded, so distinct names stay distinct.
fn escape_segment(segment: &str) -> String {
    if segment == "." || segment == ".." {
        return segment.replace('.', "%2E");
    }
    let mut out = String::with_capacity(segment.len());
    for c in segment.chars() {
        if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' ') {
            out.push(c);
        } else {
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).bytes() {
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}

fn one_line(yaml: &str) -> String {
    yaml.trim().replace('\n', ", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_values() {
        assert!(!env_disables_cache(None));
        assert!(!env_disables_cache(Some("on")));
        assert!(!env_disables_cache(Some("")));
        for v in ["off", "OFF", "0", "false", " False "] {
            assert!(env_disables_cache(Some(v)), "{v}");
        }
    }

    #[test]
    fn segments_stay_distinct() {
        assert_eq!(escape_segment("train"), "train");
        assert_eq!(escape_segment("a_b"), "a_b");
        assert_eq!(escape_segment(".."), "%2E%2E");
        assert_eq!(escape_segment("a:b"), "a%3Ab");
        assert_ne!(escape_segment("a%3Ab"), escape_segment("a:b"));
    }

    #[test]
    fn node_paths_nest() {
        let hook = CacheHook::new("/c");
        assert_eq!(hook.node_dir("epochs/0/train"), PathBuf::from("/c/epochs/0/train"));
        assert_ne!(hook.node_dir("a/b"), hook.node_dir("a_b"));
    }
}
