//! Axum web server for the pipeline visualization.

use std::collections::HashMap;
use std::string::{String, ToString};
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::{Path, State};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{StatusCode, Uri, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use super::assets::{FrontendAssets, mime_for_path};
use super::hook::VizEvent;
use super::serialization::VizGraph;

/// Current execution status of a single node.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeStatus {
    pub status: String, // "pending" | "running" | "completed" | "error"
    pub duration_ms: Option<f64>,
    pub error: Option<String>,
}

/// Per-dataset I/O timing tracked while the pipeline runs.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DatasetActivity {
    pub load_ms: Option<f64>,
    pub save_ms: Option<f64>,
}

/// Shared state for the viz server.
pub struct VizState {
    pub graph: VizGraph,
    /// `DatasetMeta` references with erased lifetime. `html()` is called at
    /// request time so previews reflect post-run state.
    /// See `collect_dataset_meta` for the safety argument.
    pub dataset_meta: HashMap<usize, &'static dyn crate::DatasetMeta>,
    pub node_statuses: Mutex<HashMap<String, NodeStatus>>,
    /// Dataset activity keyed by dataset name (from VizEvent::dataset_name).
    pub dataset_activity: Mutex<HashMap<String, DatasetActivity>>,
    pub tx: broadcast::Sender<String>,
}

// ── Route handlers ────────────────────────────────────────────────────────────

async fn get_graph(State(state): State<Arc<VizState>>) -> impl IntoResponse {
    Json(state.graph.clone())
}

async fn get_dataset_html(
    Path(id): Path<usize>,
    State(state): State<Arc<VizState>>,
) -> impl IntoResponse {
    match state.dataset_meta.get(&id) {
        Some(meta) => match meta.html() {
            Some(html) => Html(html).into_response(),
            None => (StatusCode::NOT_FOUND, "No HTML preview available").into_response(),
        },
        None => (StatusCode::NOT_FOUND, "No HTML preview available").into_response(),
    }
}

async fn get_dataset_yaml(
    Path(id): Path<usize>,
    State(state): State<Arc<VizState>>,
) -> impl IntoResponse {
    match state.dataset_meta.get(&id) {
        Some(meta) => match meta.yaml() {
            Some(yaml) => (StatusCode::OK, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], yaml).into_response(),
            None => (StatusCode::NOT_FOUND, "No YAML definition available").into_response(),
        },
        None => (StatusCode::NOT_FOUND, "No YAML definition available").into_response(),
    }
}

async fn get_status(State(state): State<Arc<VizState>>) -> impl IntoResponse {
    let statuses = state.node_statuses.lock().unwrap().clone();
    let activity = state.dataset_activity.lock().unwrap().clone();
    Json(serde_json::json!({
        "nodes": statuses,
        "datasets": activity,
    }))
}

async fn post_status(
    State(state): State<Arc<VizState>>,
    Json(event): Json<VizEvent>,
) -> StatusCode {
    // Update node status based on event kind
    use crate::viz::VizEventKind;
    match event.kind {
        VizEventKind::BeforeNodeRun | VizEventKind::BeforePipelineRun => {
            state.node_statuses.lock().unwrap().insert(
                event.node_name.clone(),
                NodeStatus { status: "running".to_string(), duration_ms: None, error: None },
            );
        }
        VizEventKind::AfterNodeRun | VizEventKind::AfterPipelineRun => {
            state.node_statuses.lock().unwrap().insert(
                event.node_name.clone(),
                NodeStatus {
                    status: "completed".to_string(),
                    duration_ms: event.duration_ms,
                    error: None,
                },
            );
        }
        VizEventKind::OnNodeError | VizEventKind::OnPipelineError => {
            state.node_statuses.lock().unwrap().insert(
                event.node_name.clone(),
                NodeStatus {
                    status: "error".to_string(),
                    duration_ms: None,
                    error: event.error.clone(),
                },
            );
        }
        VizEventKind::AfterDatasetLoaded => {
            if let Some(ds_name) = &event.dataset_name {
                let mut map = state.dataset_activity.lock().unwrap();
                let entry = map.entry(ds_name.clone()).or_default();
                entry.load_ms = event.duration_ms;
            }
        }
        VizEventKind::AfterDatasetSaved => {
            if let Some(ds_name) = &event.dataset_name {
                let mut map = state.dataset_activity.lock().unwrap();
                let entry = map.entry(ds_name.clone()).or_default();
                entry.save_ms = event.duration_ms;
            }
        }
        VizEventKind::BeforeDatasetLoaded | VizEventKind::BeforeDatasetSaved => {}
    }

    // Broadcast the raw event JSON to all WebSocket clients
    let msg = serde_json::to_string(&event).unwrap_or_default();
    let _ = state.tx.send(msg);

    StatusCode::OK
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<VizState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: Arc<VizState>) {
    let mut rx = state.tx.subscribe();
    loop {
        match rx.recv().await {
            Ok(msg) => {
                if socket.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Closed) => break,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
        }
    }
}

async fn serve_asset(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match FrontendAssets::get(path) {
        Some(content) => {
            let mime = mime_for_path(path);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(content.data.into_owned()))
                .unwrap()
                .into_response()
        }
        None => {
            // SPA fallback: serve index.html for client-side routes
            match FrontendAssets::get("index.html") {
                Some(content) => Html(
                    String::from_utf8_lossy(&content.data).into_owned(),
                )
                .into_response(),
                None => StatusCode::NOT_FOUND.into_response(),
            }
        }
    }
}

// ── Server entry point ────────────────────────────────────────────────────────

/// Build and run the viz server on the given port. Blocks until the process is killed.
pub fn start_server(state: VizState, port: u16) {
    let state = Arc::new(state);

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("viz: failed to build runtime: {e}");
            std::process::exit(1);
        }
    };

    rt.block_on(async move {
        let app = Router::new()
            .route("/api/graph", get(get_graph))
            .route("/api/dataset/{id}/html", get(get_dataset_html))
            .route("/api/dataset/{id}/yaml", get(get_dataset_yaml))
            .route("/api/status", get(get_status))
            .route("/api/status", post(post_status))
            .route("/ws", get(ws_handler))
            .fallback(serve_asset)
            .with_state(state);

        let addr = format!("0.0.0.0:{port}");
        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("viz: failed to bind {addr}: {e}");
                std::process::exit(1);
            }
        };

        println!("Viz server running at http://localhost:{port}");
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("viz: server error: {e}");
        }
    });
}
