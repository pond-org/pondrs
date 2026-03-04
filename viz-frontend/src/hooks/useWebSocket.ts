import { useEffect, useRef, useState } from 'react';
import type { VizEvent, NodeStatus, DatasetActivity } from '../api/types';

export interface LiveStatus {
  nodes: Record<string, NodeStatus>;
  datasets: Record<string, DatasetActivity>;
  connected: boolean;
  lastEvent: VizEvent | null;
}

export function useWebSocket(): LiveStatus {
  const [status, setStatus] = useState<LiveStatus>({
    nodes: {},
    datasets: {},
    connected: false,
    lastEvent: null,
  });
  const wsRef = useRef<WebSocket | null>(null);
  const retryRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  function connect() {
    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${proto}//${window.location.host}/ws`);
    wsRef.current = ws;

    ws.onopen = () =>
      setStatus(prev => ({ ...prev, connected: true }));

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

          switch (event.event_type) {
            case 'node_start':
              nodes[event.node_name] = { status: 'running', duration_ms: null, error: null };
              break;
            case 'node_end':
              nodes[event.node_name] = { status: 'completed', duration_ms: event.duration_ms, error: null };
              break;
            case 'node_error':
              nodes[event.node_name] = { status: 'error', duration_ms: null, error: event.error };
              break;
            case 'dataset_load_end':
              if (event.dataset_name) {
                const prev_ds = datasets[event.dataset_name] ?? { load_ms: null, save_ms: null };
                datasets[event.dataset_name] = { ...prev_ds, load_ms: event.duration_ms };
              }
              break;
            case 'dataset_save_end':
              if (event.dataset_name) {
                const prev_ds = datasets[event.dataset_name] ?? { load_ms: null, save_ms: null };
                datasets[event.dataset_name] = { ...prev_ds, save_ms: event.duration_ms };
              }
              break;
          }

          return { nodes, datasets, connected: true, lastEvent: event };
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
