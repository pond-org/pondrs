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

  const { nodes: nodeStatuses, datasets: datasetActivity, connected, lastEvent } = useWebSocket();

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
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh', background: '#0f0f0f', color: '#ef4444', fontFamily: 'sans-serif' }}>
        <div>
          <div style={{ fontSize: 16, fontWeight: 600, marginBottom: 8 }}>Failed to load pipeline graph</div>
          <div style={{ fontSize: 13, color: '#888' }}>{error}</div>
        </div>
      </div>
    );
  }

  const panelOpen = selection != null;

  return (
    <div style={{ width: '100vw', height: '100vh', background: '#0f0f0f', position: 'relative', overflow: 'hidden' }}>
      {/* Header */}
      <div style={{
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        height: 40,
        background: '#111',
        borderBottom: '1px solid #222',
        display: 'flex',
        alignItems: 'center',
        padding: '0 16px',
        zIndex: 5,
        gap: 12,
      }}>
        <span style={{ color: '#e5e5e5', fontWeight: 700, fontSize: 14, letterSpacing: '0.05em' }}>pondrs viz</span>
        {graph && (
          <span style={{ color: '#555', fontSize: 12 }}>
            {graph.nodes.filter(n => !n.is_pipe).length} nodes · {graph.datasets.length} datasets
          </span>
        )}
      </div>

      {/* Main canvas */}
      <div style={{
        position: 'absolute',
        top: 40,
        left: 0,
        right: panelOpen ? 480 : 0,
        bottom: 32,
        transition: 'right 0.2s ease',
      }}>
        {!graph && !error && (
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: '#555', fontSize: 13 }}>
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
          />
        )}
      </div>

      {/* Info panel */}
      <div style={{ position: 'absolute', top: 40, right: 0, bottom: 32, width: 480, pointerEvents: 'none' }}>
        <DatasetPanel selection={selection} onClose={handleClose} />
      </div>

      {/* Status bar */}
      <StatusBar connected={connected} lastEvent={lastEvent} />
    </div>
  );
}
