import { useEffect, useState, useCallback } from 'react';
import type { VizGraph } from './api/types';
import { fetchGraph } from './api/client';
import { useWebSocket } from './hooks/useWebSocket';
import { GraphView } from './components/GraphView';
import { DatasetPanel, type PanelSelection } from './components/DatasetPanel';
import { StatusBar } from './components/StatusBar';

export function App() {
  const [graph, setGraph] = useState<VizGraph | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selection, setSelection] = useState<PanelSelection | null>(null);
  const [isDark, setIsDark] = useState(true);

  const { nodes: nodeStatuses, datasets: datasetActivity, connected, lastEvent } = useWebSocket();

  useEffect(() => {
    document.documentElement.dataset.theme = isDark ? 'dark' : 'light';
  }, [isDark]);

  useEffect(() => {
    fetchGraph()
      .then(setGraph)
      .catch(e => setError(String(e)));
  }, []);

  const handleDatasetSelect = useCallback((id: number) => {
    setSelection(prev => {
      if (prev?.kind === 'dataset' && prev.id === id) return null;
      const ds = graph?.datasets.find(d => d.id === id);
      if (!ds) return null;
      return {
        kind: 'dataset',
        id,
        name: ds.name,
        is_param: ds.is_param,
        activity: datasetActivity[ds.name] ?? null,
      };
    });
  }, [graph, datasetActivity]);

  const handleNodeSelect = useCallback((name: string) => {
    setSelection(prev => {
      if (prev?.kind === 'node' && prev.name === name) return null;
      return {
        kind: 'node',
        name,
        status: nodeStatuses[name] ?? null,
      };
    });
  }, [nodeStatuses]);

  const handleClose = useCallback(() => setSelection(null), []);

  if (error) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh', background: 'var(--bg)', color: 'var(--color-error)', fontFamily: 'Inter, system-ui, sans-serif' }}>
        <div>
          <div style={{ fontSize: 24, fontWeight: 600, marginBottom: 10 }}>Failed to load pipeline graph</div>
          <div style={{ fontSize: 18, color: 'var(--text-muted)' }}>{error}</div>
        </div>
      </div>
    );
  }

  const panelOpen = selection != null;

  return (
    <div style={{ width: '100vw', height: '100vh', background: 'var(--bg)', position: 'relative', overflow: 'hidden' }}>
      {/* Header */}
      <div style={{
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        height: 56,
        background: 'var(--bg-header)',
        borderBottom: '1px solid var(--border)',
        display: 'flex',
        alignItems: 'center',
        padding: '0 20px',
        zIndex: 5,
        gap: 14,
      }}>
        <span style={{ color: 'var(--text)', fontWeight: 700, fontSize: 21, letterSpacing: '0.04em' }}>pondrs viz</span>
        {graph && (
          <span style={{ color: 'var(--text-dim)', fontSize: 17 }}>
            {graph.nodes.filter(n => !n.is_pipe).length} nodes · {graph.datasets.length} datasets
          </span>
        )}
        <div style={{ marginLeft: 'auto' }}>
          <button
            onClick={() => setIsDark(d => !d)}
            title={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
            style={{
              background: 'var(--bg-tag)',
              border: '1px solid var(--border-tag)',
              borderRadius: 8,
              color: 'var(--text-muted)',
              cursor: 'pointer',
              fontSize: 18,
              lineHeight: 1,
              padding: '4px 10px',
              transition: 'color 0.2s',
            }}
          >
            {isDark ? '☀' : '☾'}
          </button>
        </div>
      </div>

      {/* Main canvas */}
      <div style={{
        position: 'absolute',
        top: 56,
        left: 0,
        right: panelOpen ? 520 : 0,
        bottom: 48,
        transition: 'right 0.2s ease',
      }}>
        {!graph && !error && (
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-dim)', fontSize: 20 }}>
            Loading…
          </div>
        )}
        {graph && (
          <GraphView
            graph={graph}
            nodeStatuses={nodeStatuses}
            datasetActivity={datasetActivity}
            onDatasetSelect={handleDatasetSelect}
            onNodeSelect={handleNodeSelect}
            isDark={isDark}
          />
        )}
      </div>

      {/* Info panel */}
      <div style={{ position: 'absolute', top: 56, right: 0, bottom: 48, width: 520, pointerEvents: 'none' }}>
        <DatasetPanel selection={selection} onClose={handleClose} isDark={isDark} />
      </div>

      {/* Status bar */}
      <StatusBar connected={connected} lastEvent={lastEvent} />
    </div>
  );
}
