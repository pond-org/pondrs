// Types mirroring the Rust VizGraph serialization structs.

export interface VizDataset {
  id: number;
  name: string;
  type_string: string;
  is_param: boolean;
  has_html: boolean;
  has_yaml: boolean;
}

export interface VizNode {
  id: number;
  name: string;
  type_string: string;
  is_pipe: boolean;
  parent_pipe: number | null;
  pipe_children: number[];
  input_dataset_ids: number[];
  output_dataset_ids: number[];
}

export interface DatasetSummary {
  name: string;
  type_string: string;
}

export interface VizEdge {
  from_node: number;
  to_node: number;
  dataset_id: number;
}

export interface VizGraph {
  name: string;
  nodes: VizNode[];
  edges: VizEdge[];
  datasets: VizDataset[];
}

// Node execution status, mirroring NodeStatus on the server.
export type StatusKind = 'pending' | 'running' | 'completed' | 'error';

export interface NodeStatus {
  status: StatusKind;
  duration_ms: number | null;
  error: string | null;
}

export interface DatasetActivity {
  load_ms: number | null;
  save_ms: number | null;
}

export interface StatusSnapshot {
  nodes: Record<string, NodeStatus>;
  datasets: Record<string, DatasetActivity>;
}

// Event kinds — mirror Rust VizEventKind (serde snake_case = Kedro hook names).
export type VizEventKind =
  | 'before_pipeline_run'
  | 'after_pipeline_run'
  | 'on_pipeline_error'
  | 'before_node_run'
  | 'after_node_run'
  | 'on_node_error'
  | 'before_dataset_loaded'
  | 'after_dataset_loaded'
  | 'before_dataset_saved'
  | 'after_dataset_saved';

// Events pushed over WebSocket.
export interface VizEvent {
  kind: VizEventKind;
  node_name: string;
  duration_ms: number | null;
  error: string | null;
  dataset_id: number | null;
  dataset_name: string | null;
}
