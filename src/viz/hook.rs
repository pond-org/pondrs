//! VizHook: fires HTTP events to a running viz server during pipeline execution.

use std::prelude::v1::*;

use serde::{Deserialize, Serialize};

use crate::pipeline::{DatasetRef, StepInfo};
use crate::hooks::Hook;
use crate::hooks::timing::TimingTracker;

/// The kind of execution event sent from `VizHook` to the viz server.
///
/// Serializes to/from snake_case strings matching Kedro hook method names
/// (e.g. `BeforePipelineRun` ↔ `"before_pipeline_run"`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VizEventKind {
    BeforePipelineRun,
    AfterPipelineRun,
    OnPipelineError,
    BeforeNodeRun,
    AfterNodeRun,
    OnNodeError,
    BeforeDatasetLoaded,
    AfterDatasetLoaded,
    BeforeDatasetSaved,
    AfterDatasetSaved,
}

/// An event sent from `VizHook` to the viz server's POST /api/status endpoint.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VizEvent {
    pub kind: VizEventKind,
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
    timings: TimingTracker<String>,
}

impl VizHook {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            timings: TimingTracker::new(),
        }
    }

    fn send(&self, event: &VizEvent) {
        let url = format!("{}/api/status", self.base_url);
        let _ = ureq::post(&url).send_json(event);
    }

    fn ds_timing_key(ds: &DatasetRef<'_>) -> String {
        format!("ds_{}", ds.id)
    }
}

impl Hook for VizHook {
    fn before_pipeline_run(&self, p: &dyn StepInfo) {
        let name = p.name();
        self.timings.start(name.to_string());
        self.send(&VizEvent {
            kind: VizEventKind::BeforePipelineRun,
            node_name: name.to_string(),
            duration_ms: None,
            error: None,
            dataset_id: None,
            dataset_name: None,
        });
    }

    fn after_pipeline_run(&self, p: &dyn StepInfo) {
        let name = p.name();
        let duration_ms = self.timings.elapsed_ms(&name.to_string());
        self.send(&VizEvent {
            kind: VizEventKind::AfterPipelineRun,
            node_name: name.to_string(),
            duration_ms,
            error: None,
            dataset_id: None,
            dataset_name: None,
        });
    }

    fn on_pipeline_error(&self, p: &dyn StepInfo, error: &str) {
        let name = p.name();
        self.timings.elapsed_ms(&name.to_string()); // clean up timing entry
        self.send(&VizEvent {
            kind: VizEventKind::OnPipelineError,
            node_name: name.to_string(),
            duration_ms: None,
            error: Some(error.to_string()),
            dataset_id: None,
            dataset_name: None,
        });
    }

    fn before_node_run(&self, n: &dyn StepInfo) {
        let name = n.name();
        self.timings.start(name.to_string());
        self.send(&VizEvent {
            kind: VizEventKind::BeforeNodeRun,
            node_name: name.to_string(),
            duration_ms: None,
            error: None,
            dataset_id: None,
            dataset_name: None,
        });
    }

    fn after_node_run(&self, n: &dyn StepInfo) {
        let name = n.name();
        let duration_ms = self.timings.elapsed_ms(&name.to_string());
        self.send(&VizEvent {
            kind: VizEventKind::AfterNodeRun,
            node_name: name.to_string(),
            duration_ms,
            error: None,
            dataset_id: None,
            dataset_name: None,
        });
    }

    fn on_node_error(&self, n: &dyn StepInfo, error: &str) {
        let name = n.name();
        self.timings.elapsed_ms(&name.to_string()); // clean up timing entry
        self.send(&VizEvent {
            kind: VizEventKind::OnNodeError,
            node_name: name.to_string(),
            duration_ms: None,
            error: Some(error.to_string()),
            dataset_id: None,
            dataset_name: None,
        });
    }

    fn before_dataset_loaded(&self, n: &dyn StepInfo, ds: &DatasetRef<'_>) {
        self.timings.start(Self::ds_timing_key(ds));
        self.send(&VizEvent {
            kind: VizEventKind::BeforeDatasetLoaded,
            node_name: n.name().to_string(),
            duration_ms: None,
            error: None,
            dataset_id: Some(ds.id),
            dataset_name: ds.name.map(|s| s.to_string()),
        });
    }

    fn after_dataset_loaded(&self, n: &dyn StepInfo, ds: &DatasetRef<'_>) {
        let duration_ms = self.timings.elapsed_ms(&Self::ds_timing_key(ds));
        self.send(&VizEvent {
            kind: VizEventKind::AfterDatasetLoaded,
            node_name: n.name().to_string(),
            duration_ms,
            error: None,
            dataset_id: Some(ds.id),
            dataset_name: ds.name.map(|s| s.to_string()),
        });
    }

    fn before_dataset_saved(&self, n: &dyn StepInfo, ds: &DatasetRef<'_>) {
        self.timings.start(Self::ds_timing_key(ds));
        self.send(&VizEvent {
            kind: VizEventKind::BeforeDatasetSaved,
            node_name: n.name().to_string(),
            duration_ms: None,
            error: None,
            dataset_id: Some(ds.id),
            dataset_name: ds.name.map(|s| s.to_string()),
        });
    }

    fn after_dataset_saved(&self, n: &dyn StepInfo, ds: &DatasetRef<'_>) {
        let duration_ms = self.timings.elapsed_ms(&Self::ds_timing_key(ds));
        self.send(&VizEvent {
            kind: VizEventKind::AfterDatasetSaved,
            node_name: n.name().to_string(),
            duration_ms,
            error: None,
            dataset_id: Some(ds.id),
            dataset_name: ds.name.map(|s| s.to_string()),
        });
    }
}
