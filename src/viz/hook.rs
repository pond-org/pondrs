//! `VizHook`: fires HTTP events to a running viz server during pipeline execution.

use std::collections::VecDeque;
use std::prelude::v1::*;
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::pipeline::{DatasetRef, StepMeta};
use crate::hooks::Hook;
use crate::hooks::timing::TimingTracker;

/// The kind of execution event sent from `VizHook` to the viz server.
///
/// Serializes to/from `snake_case` strings matching Kedro hook method names
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
    /// Name of the step (node or pipeline) involved.
    pub node_name: String,
    pub duration_ms: Option<f64>,
    pub error: Option<String>,
    /// Present for dataset events.
    pub dataset_id: Option<usize>,
    /// Human-readable dataset name when available.
    pub dataset_name: Option<String>,
}

/// Most events [`VizHook`] holds while the viz server is unreachable; past
/// this the oldest are dropped.
const MAX_UNSENT: usize = 10_000;

/// Hook that POSTs execution events to a viz server.
///
/// Never fails the pipeline: events the server cannot be reached for are
/// held (up to [`MAX_UNSENT`]) and replayed in order once it answers, so a
/// viz server started mid-run still learns what ran before it. An event the
/// server rejects is dropped.
pub struct VizHook {
    base_url: String,
    timings: TimingTracker<String>,
    unsent: Mutex<VecDeque<VizEvent>>,
}

impl VizHook {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            timings: TimingTracker::new(),
            unsent: Mutex::new(VecDeque::new()),
        }
    }

    /// Queue `event` behind any unsent ones and send as many as the server
    /// takes. The lock is held while posting so events arrive in order even
    /// when nodes run in parallel.
    fn send(&self, event: &VizEvent) {
        let url = format!("{}/api/status", self.base_url);
        let mut unsent = self.unsent.lock().unwrap_or_else(PoisonError::into_inner);
        if unsent.len() == MAX_UNSENT {
            unsent.pop_front();
        }
        unsent.push_back(event.clone());
        while let Some(next) = unsent.front() {
            match ureq::post(&url).send_json(next) {
                // Delivered, or refused for good: either way it is done.
                Ok(_) | Err(ureq::Error::StatusCode(_)) => {
                    unsent.pop_front();
                }
                // Server unreachable: keep everything for the next attempt.
                Err(_) => break,
            }
        }
    }

    fn ds_timing_key(ds: &DatasetRef<'_>) -> String {
        format!("ds_{}", ds.id)
    }
}

impl Hook for VizHook {
    fn before_pipeline_run(&self, p: &dyn StepMeta) -> Result<crate::hooks::HookControl, crate::hooks::HookAbort> {
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
        Ok(crate::hooks::HookControl::Continue)
    }

    fn after_pipeline_run(&self, p: &dyn StepMeta) -> Result<(), crate::hooks::HookAbort> {
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
        Ok(())
    }

    fn on_pipeline_error(&self, p: &dyn StepMeta, error: &str) {
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

    fn before_node_run(&self, n: &dyn StepMeta) -> Result<crate::hooks::HookControl, crate::hooks::HookAbort> {
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
        Ok(crate::hooks::HookControl::Continue)
    }

    fn after_node_run(&self, n: &dyn StepMeta, _skipped: bool) -> Result<(), crate::hooks::HookAbort> {
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
        Ok(())
    }

    fn on_node_error(&self, n: &dyn StepMeta, error: &str) {
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

    fn before_dataset_loaded(&self, n: &dyn StepMeta, ds: &DatasetRef<'_>) -> Result<crate::hooks::HookControl, crate::hooks::HookAbort> {
        self.timings.start(Self::ds_timing_key(ds));
        self.send(&VizEvent {
            kind: VizEventKind::BeforeDatasetLoaded,
            node_name: n.name().to_string(),
            duration_ms: None,
            error: None,
            dataset_id: Some(ds.id),
            dataset_name: ds.name.map(ToString::to_string),
        });
        Ok(crate::hooks::HookControl::Continue)
    }

    fn after_dataset_loaded(&self, n: &dyn StepMeta, ds: &DatasetRef<'_>, _value: &dyn core::any::Any) -> Result<(), crate::hooks::HookAbort> {
        let duration_ms = self.timings.elapsed_ms(&Self::ds_timing_key(ds));
        self.send(&VizEvent {
            kind: VizEventKind::AfterDatasetLoaded,
            node_name: n.name().to_string(),
            duration_ms,
            error: None,
            dataset_id: Some(ds.id),
            dataset_name: ds.name.map(ToString::to_string),
        });
        Ok(())
    }

    fn before_dataset_saved(&self, n: &dyn StepMeta, ds: &DatasetRef<'_>, _value: &dyn core::any::Any) -> Result<crate::hooks::HookControl, crate::hooks::HookAbort> {
        self.timings.start(Self::ds_timing_key(ds));
        self.send(&VizEvent {
            kind: VizEventKind::BeforeDatasetSaved,
            node_name: n.name().to_string(),
            duration_ms: None,
            error: None,
            dataset_id: Some(ds.id),
            dataset_name: ds.name.map(ToString::to_string),
        });
        Ok(crate::hooks::HookControl::Continue)
    }

    fn after_dataset_saved(&self, n: &dyn StepMeta, ds: &DatasetRef<'_>) -> Result<(), crate::hooks::HookAbort> {
        let duration_ms = self.timings.elapsed_ms(&Self::ds_timing_key(ds));
        self.send(&VizEvent {
            kind: VizEventKind::AfterDatasetSaved,
            node_name: n.name().to_string(),
            duration_ms,
            error: None,
            dataset_id: Some(ds.id),
            dataset_name: ds.name.map(ToString::to_string),
        });
        Ok(())
    }
}
