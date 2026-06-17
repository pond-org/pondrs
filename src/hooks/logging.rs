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
    fn before_pipeline_run(&self, p: &dyn StepInfo) -> Result<super::HookControl, super::HookAbort> {
        info!("[pipeline] {} - starting", p.name());
        self.timings.start(item_key(p));
        Ok(super::HookControl::Continue)
    }

    fn after_pipeline_run(&self, p: &dyn StepInfo) -> Result<(), super::HookAbort> {
        if let Some(ms) = self.timings.elapsed_ms(&item_key(p)) {
            info!("[pipeline] {} - completed ({:.1}ms)", p.name(), ms);
        } else {
            info!("[pipeline] {} - completed", p.name());
        }
        Ok(())
    }

    fn on_pipeline_error(&self, p: &dyn StepInfo, error: &str) {
        self.timings.elapsed_ms(&item_key(p)); // clean up timing entry
        warn!("[pipeline] {} - error: {}", p.name(), error);
    }

    fn before_node_run(&self, n: &dyn StepInfo) -> Result<super::HookControl, super::HookAbort> {
        info!("[node] {} - starting", n.name());
        self.timings.start(item_key(n));
        Ok(super::HookControl::Continue)
    }

    fn after_node_run(&self, n: &dyn StepInfo, skipped: bool) -> Result<(), super::HookAbort> {
        if skipped {
            self.timings.elapsed_ms(&item_key(n));
            info!("[node] {} - skipped (cached)", n.name());
        } else if let Some(ms) = self.timings.elapsed_ms(&item_key(n)) {
            info!("[node] {} - completed ({:.1}ms)", n.name(), ms);
        } else {
            info!("[node] {} - completed", n.name());
        }
        Ok(())
    }

    fn on_node_error(&self, n: &dyn StepInfo, error: &str) {
        self.timings.elapsed_ms(&item_key(n)); // clean up timing entry
        warn!("[node] {} - error: {}", n.name(), error);
    }

    fn before_dataset_loaded(&self, _n: &dyn StepInfo, ds: &DatasetRef) -> Result<super::HookControl, super::HookAbort> {
        debug!("  loading {}", ds_name(ds));
        self.timings.start(ds.id);
        Ok(super::HookControl::Continue)
    }

    fn after_dataset_loaded(&self, _n: &dyn StepInfo, ds: &DatasetRef, _value: &dyn core::any::Any) -> Result<(), super::HookAbort> {
        if let Some(ms) = self.timings.elapsed_ms(&ds.id) {
            debug!("  loaded {} ({:.1}ms)", ds_name(ds), ms);
        } else {
            debug!("  loaded {}", ds_name(ds));
        }
        Ok(())
    }

    fn before_dataset_saved(&self, _n: &dyn StepInfo, ds: &DatasetRef, _value: &dyn core::any::Any) -> Result<super::HookControl, super::HookAbort> {
        debug!("  saving {}", ds_name(ds));
        self.timings.start(ds.id);
        Ok(super::HookControl::Continue)
    }

    fn after_dataset_saved(&self, _n: &dyn StepInfo, ds: &DatasetRef) -> Result<(), super::HookAbort> {
        if let Some(ms) = self.timings.elapsed_ms(&ds.id) {
            debug!("  saved {} ({:.1}ms)", ds_name(ds), ms);
        } else {
            debug!("  saved {}", ds_name(ds));
        }
        Ok(())
    }
}
