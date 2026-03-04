import { useEffect, useState, useCallback } from 'react';
import type { VizGraph } from './api/types';
import { fetchGraph } from './api/client';
import { useWebSocket } from './hooks/useWebSocket';
import { GraphView, type CenterRequest } from './components/GraphView';
import { DatasetPanel, type PanelSelection } from './components/DatasetPanel';
import { LeftPanel } from './components/LeftPanel';
import { StatusBar } from './components/StatusBar';

const LEFT_W = 240;

export function App() {
  const [graph, setGraph] = useState<VizGraph | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selection, setSelection] = useState<PanelSelection | null>(null);
  const [isDark, setIsDark] = useState(true);
  const [leftOpen, setLeftOpen] = useState(true);
  const [centerRequest, setCenterRequest] = useState<CenterRequest | null>(null);

  const { nodes: nodeStatuses, datasets: datasetActivity, connected, lastEvent, runCount, reconnectCount } = useWebSocket();

  useEffect(() => {
    document.documentElement.dataset.theme = isDark ? 'dark' : 'light';
  }, [isDark]);

  // Fetch (or re-fetch) graph on initial load and on every reconnection
  useEffect(() => {
    fetchGraph()
      .then(setGraph)
      .catch(e => setError(String(e)));
  }, [reconnectCount]);

  const handleDatasetSelect = useCallback((id: number) => {
    setSelection(prev => {
      if (prev?.kind === 'dataset' && prev.id === id) return null;
      const ds = graph?.datasets.find(d => d.id === id);
      if (!ds) return null;
      return {
        kind: 'dataset',
        id,
        name: ds.name,
        type_string: ds.type_string,
        is_param: ds.is_param,
        activity: datasetActivity[ds.name] ?? null,
      };
    });
  }, [graph, datasetActivity]);

  const handleNodeSelect = useCallback((name: string) => {
    setSelection(prev => {
      if (prev?.kind === 'node' && prev.name === name) return null;
      const node = graph?.nodes.find(n => n.name === name);
      return { kind: 'node', name, type_string: node?.type_string ?? '', status: nodeStatuses[name] ?? null };
    });
  }, [graph, nodeStatuses]);

  const handleLeftSelect = useCallback((rfId: string, sel: PanelSelection) => {
    setSelection(sel);
    setCenterRequest(prev => ({ id: rfId, tick: (prev?.tick ?? 0) + 1 }));
  }, []);

  const handlePaneClick = useCallback(() => {
    setSelection(null);
  }, []);

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
  const canvasLeft = leftOpen ? LEFT_W : 0;
  const canvasRight = panelOpen ? 520 : 0;

  return (
    <div style={{ width: '100vw', height: '100vh', background: 'var(--bg)', position: 'relative', overflow: 'hidden' }}>
      {/* Header */}
      <div style={{
        position: 'absolute',
        top: 0, left: 0, right: 0,
        height: 56,
        background: 'var(--bg-header)',
        borderBottom: '1px solid var(--border)',
        display: 'flex',
        alignItems: 'center',
        padding: '0 16px',
        zIndex: 5,
        gap: 10,
      }}>
        {/* Sidebar toggle */}
        <button
          onClick={() => setLeftOpen(o => !o)}
          title={leftOpen ? 'Hide sidebar' : 'Show sidebar'}
          style={{
            background: 'var(--bg-tag)',
            border: '1px solid var(--border-tag)',
            borderRadius: 8,
            color: 'var(--text-muted)',
            cursor: 'pointer',
            fontSize: 18,
            lineHeight: 1,
            padding: '4px 10px',
            flexShrink: 0,
          }}
        >
          ☰
        </button>

        <span style={{ color: 'var(--text)', fontWeight: 700, fontSize: 21, letterSpacing: '0.04em' }}>🤔 pondrs viz</span>

        <div style={{ flex: 1 }} />

        {graph && (
          <span style={{ color: 'var(--text-sub)', fontWeight: 600, fontSize: 19 }}>
            {graph.name || 'pipeline'}
            <span style={{ color: 'var(--text-dim)', fontWeight: 400, fontSize: 15, marginLeft: 10 }}>
              {graph.nodes.filter(n => !n.is_pipe).length} nodes · {graph.datasets.length} datasets
            </span>
          </span>
        )}

        <div style={{ flex: 1 }} />

        <div>
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
            }}
          >
            {isDark ? '☀' : '☾'}
          </button>
        </div>
      </div>

      {/* Left sidebar */}
      <div style={{
        position: 'absolute',
        top: 56, bottom: 48, left: 0,
        width: LEFT_W,
        transform: leftOpen ? 'translateX(0)' : `translateX(-${LEFT_W}px)`,
        transition: 'transform 0.2s ease',
        zIndex: 4,
      }}>
        <LeftPanel
          graph={graph}
          selection={selection}
          nodeStatuses={nodeStatuses}
          datasetActivity={datasetActivity}
          onSelect={handleLeftSelect}
        />
      </div>

      {/* Main canvas */}
      <div style={{
        position: 'absolute',
        top: 56,
        left: canvasLeft,
        right: canvasRight,
        bottom: 48,
        transition: 'left 0.2s ease, right 0.2s ease',
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
            centerRequest={centerRequest}
            onPaneClick={handlePaneClick}
          />
        )}
      </div>

      {/* Right info panel */}
      <div style={{ position: 'absolute', top: 56, right: 0, bottom: 48, width: 520, pointerEvents: 'none' }}>
        <DatasetPanel selection={selection} onClose={handleClose} isDark={isDark} reconnectCount={reconnectCount} />
      </div>

      <StatusBar connected={connected} lastEvent={lastEvent} graph={graph} nodeStatuses={nodeStatuses} runCount={runCount} />
    </div>
  );
}
