//! Pipeline visualization: axum server, serialization, and VizHook.

pub mod assets;
pub mod hook;
pub mod serialization;
pub mod server;

pub use hook::VizHook;
pub use serialization::{VizGraph, collect_dataset_meta, viz_graph_from};
pub use server::{VizState, start_server};
