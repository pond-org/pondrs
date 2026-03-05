//! Logging hook for pipeline execution with timing.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use log::{debug, info, warn};

use crate::core::{DatasetRef, PipelineInfo};

use super::Hook;

pub struct LoggingHook {
    timings: Mutex<HashMap<usize, Instant>>,
}

impl LoggingHook {
    pub fn new() -> Self {
        Self {
            timings: Mutex::new(HashMap::new()),
        }
    }

    fn start_timing(&self, key: usize) {
        self.timings.lock().unwrap().insert(key, Instant::now());
    }

    fn elapsed_ms(&self, key: usize) -> Option<f64> {
        self.timings
            .lock()
            .unwrap()
            .remove(&key)
            .map(|start| start.elapsed().as_secs_f64() * 1000.0)
    }
}

impl Default for LoggingHook {
    fn default() -> Self {
        Self::new()
    }
}

/// Use the data pointer of a trait object as a unique key for timing.
fn item_key(item: &dyn PipelineInfo) -> usize {
    item as *const dyn PipelineInfo as *const () as usize
}

fn ds_name<'a>(ds: &'a DatasetRef) -> &'a str {
    ds.name.unwrap_or("<unknown>")
}

impl Hook for LoggingHook {
    fn before_pipeline_run(&self, p: &dyn PipelineInfo) {
        info!("[pipeline] {} - starting", p.get_name());
        self.start_timing(item_key(p));
    }

    fn after_pipeline_run(&self, p: &dyn PipelineInfo) {
        if let Some(ms) = self.elapsed_ms(item_key(p)) {
            info!("[pipeline] {} - completed ({:.1}ms)", p.get_name(), ms);
        } else {
            info!("[pipeline] {} - completed", p.get_name());
        }
    }

    fn on_pipeline_error(&self, p: &dyn PipelineInfo, error: &str) {
        self.elapsed_ms(item_key(p)); // clean up timing entry
        warn!("[pipeline] {} - error: {}", p.get_name(), error);
    }

    fn before_node_run(&self, n: &dyn PipelineInfo) {
        info!("[node] {} - starting", n.get_name());
        self.start_timing(item_key(n));
    }

    fn after_node_run(&self, n: &dyn PipelineInfo) {
        if let Some(ms) = self.elapsed_ms(item_key(n)) {
            info!("[node] {} - completed ({:.1}ms)", n.get_name(), ms);
        } else {
            info!("[node] {} - completed", n.get_name());
        }
    }

    fn on_node_error(&self, n: &dyn PipelineInfo, error: &str) {
        self.elapsed_ms(item_key(n)); // clean up timing entry
        warn!("[node] {} - error: {}", n.get_name(), error);
    }

    fn before_dataset_load(&self, _n: &dyn PipelineInfo, ds: &DatasetRef) {
        debug!("  loading {}", ds_name(ds));
        self.start_timing(ds.id);
    }

    fn after_dataset_load(&self, _n: &dyn PipelineInfo, ds: &DatasetRef) {
        if let Some(ms) = self.elapsed_ms(ds.id) {
            debug!("  loaded {} ({:.1}ms)", ds_name(ds), ms);
        } else {
            debug!("  loaded {}", ds_name(ds));
        }
    }

    fn before_dataset_save(&self, _n: &dyn PipelineInfo, ds: &DatasetRef) {
        debug!("  saving {}", ds_name(ds));
        self.start_timing(ds.id);
    }

    fn after_dataset_save(&self, _n: &dyn PipelineInfo, ds: &DatasetRef) {
        if let Some(ms) = self.elapsed_ms(ds.id) {
            debug!("  saved {} ({:.1}ms)", ds_name(ds), ms);
        } else {
            debug!("  saved {}", ds_name(ds));
        }
    }
}
