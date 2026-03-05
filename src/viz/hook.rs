//! VizHook: fires HTTP events to a running viz server during pipeline execution.

use std::collections::HashMap;
use std::prelude::v1::*;
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::core::{DatasetRef, PipelineInfo};
use crate::hooks::Hook;

/// An event sent from `VizHook` to the viz server's POST /api/status endpoint.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VizEvent {
    pub event_type: String,
    /// Name of the pipeline item (node or pipeline) involved.
    pub node_name: String,
    pub duration_ms: Option<f64>,
    pub error: Option<String>,
    /// Present for dataset events.
    pub dataset_id: Option<usize>,
    /// Human-readable dataset name when available.
    pub dataset_name: Option<String>,
}

/// Hook that POSTs execution events to a viz server.
///
/// Fire-and-forget: HTTP errors are silently ignored so a down viz server
/// never crashes the pipeline.
pub struct VizHook {
    base_url: String,
    /// Timing keys: node names + dataset IDs (as strings).
    timings: Mutex<HashMap<String, Instant>>,
}

impl VizHook {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            timings: Mutex::new(HashMap::new()),
        }
    }

    fn send(&self, event: &VizEvent) {
        let url = format!("{}/api/status", self.base_url);
        let _ = ureq::post(&url).send_json(event);
    }

    fn start_timing(&self, key: &str) {
        self.timings
            .lock()
            .unwrap()
            .insert(key.to_string(), Instant::now());
    }

    fn elapsed_ms(&self, key: &str) -> Option<f64> {
        self.timings
            .lock()
            .unwrap()
            .remove(key)
            .map(|start| start.elapsed().as_secs_f64() * 1000.0)
    }

    fn ds_timing_key(ds: &DatasetRef<'_>) -> String {
        format!("ds_{}", ds.id)
    }
}

impl Hook for VizHook {
    fn before_pipeline_run(&self, p: &dyn PipelineInfo) {
        let name = p.get_name();
        self.start_timing(name);
        self.send(&VizEvent {
            event_type: "pipeline_start".to_string(),
            node_name: name.to_string(),
            duration_ms: None,
            error: None,
            dataset_id: None,
            dataset_name: None,
        });
    }

    fn after_pipeline_run(&self, p: &dyn PipelineInfo) {
        let name = p.get_name();
        let duration_ms = self.elapsed_ms(name);
        self.send(&VizEvent {
            event_type: "pipeline_end".to_string(),
            node_name: name.to_string(),
            duration_ms,
            error: None,
            dataset_id: None,
            dataset_name: None,
        });
    }

    fn on_pipeline_error(&self, p: &dyn PipelineInfo, error: &str) {
        let name = p.get_name();
        self.elapsed_ms(name); // clean up timing entry
        self.send(&VizEvent {
            event_type: "pipeline_error".to_string(),
            node_name: name.to_string(),
            duration_ms: None,
            error: Some(error.to_string()),
            dataset_id: None,
            dataset_name: None,
        });
    }

    fn before_node_run(&self, n: &dyn PipelineInfo) {
        let name = n.get_name();
        self.start_timing(name);
        self.send(&VizEvent {
            event_type: "node_start".to_string(),
            node_name: name.to_string(),
            duration_ms: None,
            error: None,
            dataset_id: None,
            dataset_name: None,
        });
    }

    fn after_node_run(&self, n: &dyn PipelineInfo) {
        let name = n.get_name();
        let duration_ms = self.elapsed_ms(name);
        self.send(&VizEvent {
            event_type: "node_end".to_string(),
            node_name: name.to_string(),
            duration_ms,
            error: None,
            dataset_id: None,
            dataset_name: None,
        });
    }

    fn on_node_error(&self, n: &dyn PipelineInfo, error: &str) {
        let name = n.get_name();
        self.elapsed_ms(name); // clean up timing entry
        self.send(&VizEvent {
            event_type: "node_error".to_string(),
            node_name: name.to_string(),
            duration_ms: None,
            error: Some(error.to_string()),
            dataset_id: None,
            dataset_name: None,
        });
    }

    fn before_dataset_load(&self, n: &dyn PipelineInfo, ds: &DatasetRef<'_>) {
        self.start_timing(&Self::ds_timing_key(ds));
        self.send(&VizEvent {
            event_type: "dataset_load_start".to_string(),
            node_name: n.get_name().to_string(),
            duration_ms: None,
            error: None,
            dataset_id: Some(ds.id),
            dataset_name: ds.name.map(|s| s.to_string()),
        });
    }

    fn after_dataset_load(&self, n: &dyn PipelineInfo, ds: &DatasetRef<'_>) {
        let duration_ms = self.elapsed_ms(&Self::ds_timing_key(ds));
        self.send(&VizEvent {
            event_type: "dataset_load_end".to_string(),
            node_name: n.get_name().to_string(),
            duration_ms,
            error: None,
            dataset_id: Some(ds.id),
            dataset_name: ds.name.map(|s| s.to_string()),
        });
    }

    fn before_dataset_save(&self, n: &dyn PipelineInfo, ds: &DatasetRef<'_>) {
        self.start_timing(&Self::ds_timing_key(ds));
        self.send(&VizEvent {
            event_type: "dataset_save_start".to_string(),
            node_name: n.get_name().to_string(),
            duration_ms: None,
            error: None,
            dataset_id: Some(ds.id),
            dataset_name: ds.name.map(|s| s.to_string()),
        });
    }

    fn after_dataset_save(&self, n: &dyn PipelineInfo, ds: &DatasetRef<'_>) {
        let duration_ms = self.elapsed_ms(&Self::ds_timing_key(ds));
        self.send(&VizEvent {
            event_type: "dataset_save_end".to_string(),
            node_name: n.get_name().to_string(),
            duration_ms,
            error: None,
            dataset_id: Some(ds.id),
            dataset_name: ds.name.map(|s| s.to_string()),
        });
    }
}
