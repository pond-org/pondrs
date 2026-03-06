import { useEffect, useRef, useState } from 'react';
import type { VizEvent, NodeStatus, DatasetActivity } from '../api/types';

export interface LiveStatus {
  nodes: Record<string, NodeStatus>;
  datasets: Record<string, DatasetActivity>;
  connected: boolean;
  lastEvent: VizEvent | null;
  runCount: number;
  reconnectCount: number;
}

export function useWebSocket(): LiveStatus {
  const [status, setStatus] = useState<LiveStatus>({
    nodes: {},
    datasets: {},
    connected: false,
    lastEvent: null,
    runCount: 0,
    reconnectCount: 0,
  });
  const wsRef = useRef<WebSocket | null>(null);
  const retryRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  function connect() {
    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${proto}//${window.location.host}/ws`);
    wsRef.current = ws;

    ws.onopen = () =>
      setStatus(prev => ({ ...prev, connected: true, reconnectCount: prev.reconnectCount + 1 }));

    ws.onclose = () => {
      setStatus(prev => ({ ...prev, connected: false }));
      retryRef.current = setTimeout(connect, 2000);
    };

    ws.onerror = () => ws.close();

    ws.onmessage = (evt) => {
      try {
        const event: VizEvent = JSON.parse(evt.data);
        setStatus(prev => {
          const nodes = { ...prev.nodes };
          const datasets = { ...prev.datasets };

          let runCount = prev.runCount;

          switch (event.kind) {
            case 'before_node_run': {
              // Detect new run: either first-ever before_node_run (empty tracking)
              // or a node that already ran before (re-run of the pipeline)
              const isFirst = Object.keys(nodes).length === 0;
              const isRerun = event.node_name in nodes;
              if (isFirst || isRerun) {
                for (const key of Object.keys(nodes)) delete nodes[key];
                for (const key of Object.keys(datasets)) delete datasets[key];
                runCount++;
              }
              nodes[event.node_name] = { status: 'running', duration_ms: null, error: null };
              break;
            }
            case 'after_node_run':
              nodes[event.node_name] = { status: 'completed', duration_ms: event.duration_ms, error: null };
              break;
            case 'on_node_error':
              nodes[event.node_name] = { status: 'error', duration_ms: null, error: event.error };
              break;
            case 'before_pipeline_run':
              nodes[event.node_name] = { status: 'running', duration_ms: null, error: null };
              break;
            case 'after_pipeline_run':
              nodes[event.node_name] = { status: 'completed', duration_ms: event.duration_ms, error: null };
              break;
            case 'on_pipeline_error':
              nodes[event.node_name] = { status: 'error', duration_ms: null, error: event.error };
              break;
            case 'after_dataset_loaded':
              if (event.dataset_name) {
                const prev_ds = datasets[event.dataset_name] ?? { load_ms: null, save_ms: null };
                datasets[event.dataset_name] = { ...prev_ds, load_ms: event.duration_ms };
              }
              break;
            case 'after_dataset_saved':
              if (event.dataset_name) {
                const prev_ds = datasets[event.dataset_name] ?? { load_ms: null, save_ms: null };
                datasets[event.dataset_name] = { ...prev_ds, save_ms: event.duration_ms };
              }
              break;
          }

          return { nodes, datasets, connected: true, lastEvent: event, runCount, reconnectCount: prev.reconnectCount };
        });
      } catch {
        // ignore malformed messages
      }
    };
  }

  useEffect(() => {
    connect();
    return () => {
      if (retryRef.current) clearTimeout(retryRef.current);
      wsRef.current?.close();
    };
  }, []);

  return status;
}
