//! Logging hook for pipeline execution with timing.

use log::{debug, info, warn};

use crate::pipeline::{DatasetRef, StepInfo};

use super::Hook;
use super::timing::TimingTracker;

/// Hook that logs pipeline and node lifecycle events using the `log` crate.
///
/// Tracks timing for each item and includes duration in completion messages.
pub struct LoggingHook {
    timings: TimingTracker<usize>,
}

impl LoggingHook {
    pub fn new() -> Self {
        Self {
            timings: TimingTracker::new(),
        }
    }
}

impl Default for LoggingHook {
    fn default() -> Self {
        Self::new()
    }
}

/// Use the data pointer of a trait object as a unique key for timing.
fn item_key(item: &dyn StepInfo) -> usize {
    item as *const dyn StepInfo as *const () as usize
}

fn ds_name<'a>(ds: &'a DatasetRef) -> &'a str {
    ds.name.unwrap_or("<unknown>")
}

impl Hook for LoggingHook {
    fn before_pipeline_run(&self, p: &dyn StepInfo) {
        info!("[pipeline] {} - starting", p.name());
        self.timings.start(item_key(p));
    }

    fn after_pipeline_run(&self, p: &dyn StepInfo) {
        if let Some(ms) = self.timings.elapsed_ms(&item_key(p)) {
            info!("[pipeline] {} - completed ({:.1}ms)", p.name(), ms);
        } else {
            info!("[pipeline] {} - completed", p.name());
        }
    }

    fn on_pipeline_error(&self, p: &dyn StepInfo, error: &str) {
        self.timings.elapsed_ms(&item_key(p)); // clean up timing entry
        warn!("[pipeline] {} - error: {}", p.name(), error);
    }

    fn before_node_run(&self, n: &dyn StepInfo) {
        info!("[node] {} - starting", n.name());
        self.timings.start(item_key(n));
    }

    fn after_node_run(&self, n: &dyn StepInfo) {
        if let Some(ms) = self.timings.elapsed_ms(&item_key(n)) {
            info!("[node] {} - completed ({:.1}ms)", n.name(), ms);
        } else {
            info!("[node] {} - completed", n.name());
        }
    }

    fn on_node_error(&self, n: &dyn StepInfo, error: &str) {
        self.timings.elapsed_ms(&item_key(n)); // clean up timing entry
        warn!("[node] {} - error: {}", n.name(), error);
    }

    fn before_dataset_loaded(&self, _n: &dyn StepInfo, ds: &DatasetRef) {
        debug!("  loading {}", ds_name(ds));
        self.timings.start(ds.id);
    }

    fn after_dataset_loaded(&self, _n: &dyn StepInfo, ds: &DatasetRef) {
        if let Some(ms) = self.timings.elapsed_ms(&ds.id) {
            debug!("  loaded {} ({:.1}ms)", ds_name(ds), ms);
        } else {
            debug!("  loaded {}", ds_name(ds));
        }
    }

    fn before_dataset_saved(&self, _n: &dyn StepInfo, ds: &DatasetRef) {
        debug!("  saving {}", ds_name(ds));
        self.timings.start(ds.id);
    }

    fn after_dataset_saved(&self, _n: &dyn StepInfo, ds: &DatasetRef) {
        if let Some(ms) = self.timings.elapsed_ms(&ds.id) {
            debug!("  saved {} ({:.1}ms)", ds_name(ds), ms);
        } else {
            debug!("  saved {}", ds_name(ds));
        }
    }
}
