//! Retention hook: deletes older saves of a sequence of datasets.

use std::prelude::v1::*;
use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;

use log::{info, warn};

use crate::datasets::Remover;
use crate::pipeline::{DatasetRef, StepMeta};

use super::{Hook, HookAbort};

/// Keeps only the most recent saves among datasets whose names match a pattern,
/// deleting older ones as new ones are saved.
///
/// Built for write-through checkpoints in an epoch chain, where every epoch
/// persists its weights and N full checkpoints would fill a disk:
///
/// ```rust,ignore
/// RetentionHook::new("catalog.epochs.*.weights", 3).keep_every(10)
/// ```
///
/// The pattern is a full dataset name as hooks see it (rooted at `catalog.`),
/// where `*` matches exactly one dotted segment.
/// Matching saves are queued in save order, and once more than `keep_last` are
/// queued the oldest is removed through its [`Dataset::remover`]. A matching
/// dataset without a remover is skipped with a warning, logged once per dataset.
/// With [`keep_every`](Self::keep_every),
/// a save whose first numeric `*` segment is a multiple of `n` is never
/// removed, leaving resume points further back than `keep_last`; it still
/// counts as one of the last `keep_last` while it is recent.
///
/// Things to size `keep_last` against:
/// - A removed value is gone for every later reader, so `keep_last` must cover
///   the furthest-behind consumer — with `ParallelRunner`, e.g. a per-epoch
///   eval node can still be pending when the next epoch saves.
/// - `CacheHook` skips a node on its cache key without checking that the
///   outputs still exist. Resuming after a crash works, because the newest
///   checkpoints are the ones kept; a change that re-runs an epoch whose input
///   was removed fails with that input's load error.
/// - Only saves of the current run are queued, so up to `keep_last`
///   checkpoints left by an earlier run are not removed by this one.
///
/// Removal failures are logged and do not stop the pipeline.
///
/// [`Dataset::remover`]: crate::datasets::Dataset::remover
pub struct RetentionHook {
    pattern: String,
    keep_last: usize,
    keep_every: Option<usize>,
    /// Matching saves in save order; `None` marks one `keep_every` protects.
    queue: Mutex<VecDeque<(String, Option<Remover>)>>,
    /// Matching datasets already warned about for having no remover.
    warned: Mutex<HashSet<String>>,
}

impl RetentionHook {
    /// Keeps the `keep_last` most recent saves of datasets matching `pattern`.
    ///
    /// # Panics
    ///
    /// If `keep_last` is 0: that would remove each value as soon as it is saved,
    /// before any node could read it.
    pub fn new(pattern: impl Into<String>, keep_last: usize) -> Self {
        assert!(keep_last > 0, "RetentionHook: keep_last must be at least 1");
        Self {
            pattern: pattern.into(),
            keep_last,
            keep_every: None,
            queue: Mutex::new(VecDeque::new()),
            warned: Mutex::new(HashSet::new()),
        }
    }

    /// Additionally never removes saves whose index is a multiple of `n`.
    ///
    /// # Panics
    ///
    /// If `n` is 0.
    #[must_use]
    pub fn keep_every(mut self, n: usize) -> Self {
        assert!(n > 0, "RetentionHook: keep_every must be at least 1");
        self.keep_every = Some(n);
        self
    }

    /// Matches `name` against the pattern. On a match, returns whether
    /// `keep_every` protects it, judged by the first `*` segment that parses as
    /// an index.
    fn classify(&self, name: &str) -> Option<bool> {
        let mut index = None;
        let mut pattern = self.pattern.split('.');
        let mut segments = name.split('.');
        loop {
            match (pattern.next(), segments.next()) {
                (None, None) => {
                    return Some(matches!((self.keep_every, index), (Some(every), Some(i)) if i % every == 0));
                }
                (Some("*"), Some(seg)) => {
                    if index.is_none() {
                        index = seg.parse::<usize>().ok();
                    }
                }
                (Some(p), Some(seg)) if p == seg => {}
                _ => return None,
            }
        }
    }
}

impl Hook for RetentionHook {
    fn after_dataset_saved(&self, _n: &dyn StepMeta, ds: &DatasetRef) -> Result<(), HookAbort> {
        let Some(name) = ds.name else { return Ok(()) };
        let Some(protected) = self.classify(name) else { return Ok(()) };
        let Some(remover) = ds.meta.remover() else {
            let mut warned = self.warned.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if warned.insert(name.to_owned()) {
                warn!(
                    "[retention] {name} matches '{}' but {} has no remover; it will not be removed",
                    self.pattern,
                    ds.meta.type_string(),
                );
            }
            return Ok(());
        };

        let evicted: Vec<_> = {
            let mut queue = self.queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            // A protected save still takes a place in the window; it is just
            // never removed when it leaves it.
            queue.push_back((name.to_owned(), (!protected).then_some(remover)));
            let excess = queue.len().saturating_sub(self.keep_last);
            queue.drain(..excess).collect()
        };
        // Remove outside the lock: it is file IO.
        for (name, remove) in evicted {
            let Some(remove) = remove else { continue };
            match remove() {
                Ok(()) => info!("[retention] removed {name}"),
                Err(e) => warn!("[retention] failed to remove {name}: {e}"),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_matching() {
        let hook = RetentionHook::new("catalog.epochs.*.weights", 1).keep_every(2);
        assert_eq!(hook.classify("catalog.epochs.3.weights"), Some(false));
        assert_eq!(hook.classify("catalog.epochs.4.weights"), Some(true));
        assert_eq!(hook.classify("catalog.epochs.last.weights"), Some(false));
        assert_eq!(hook.classify("catalog.epochs.3.state"), None);
        assert_eq!(hook.classify("catalog.epochs.3"), None);
        assert_eq!(hook.classify("catalog.epochs.3.weights.x"), None);
        assert_eq!(hook.classify("catalog.other.3.weights"), None);
    }

    #[test]
    fn dataset_without_remover_is_skipped_and_warned_once() {
        use crate::datasets::MemoryDataset;

        let hook = RetentionHook::new("catalog.*", 1);
        let (a, b) = (MemoryDataset::<i32>::new(), MemoryDataset::<i32>::new());
        let node = crate::Node { name: "n", input: (), output: (), func: || () };
        for (ds, name) in [(&a, "catalog.a"), (&a, "catalog.a"), (&b, "catalog.b")] {
            let ds = DatasetRef { name: Some(name), ..DatasetRef::from_ref(ds) };
            hook.after_dataset_saved(&node, &ds).unwrap();
        }
        assert!(hook.queue.lock().unwrap().is_empty());
        assert_eq!(hook.warned.lock().unwrap().len(), 2);
    }
}
