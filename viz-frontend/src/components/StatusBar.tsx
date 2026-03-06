import type { VizEvent, VizGraph, NodeStatus } from '../api/types';

interface Props {
  connected: boolean;
  lastEvent: VizEvent | null;
  graph: VizGraph | null;
  nodeStatuses: Record<string, NodeStatus>;
  runCount: number;
}

export function StatusBar({ connected, lastEvent, graph, nodeStatuses, runCount }: Props) {
  // Compute progress from leaf nodes only
  const leafNodes = graph?.nodes.filter(n => !n.is_pipe) ?? [];
  const total = leafNodes.length;
  const completed = leafNodes.filter(n => nodeStatuses[n.name]?.status === 'completed').length;
  const running = leafNodes.filter(n => nodeStatuses[n.name]?.status === 'running').length;
  const errors = leafNodes.filter(n => nodeStatuses[n.name]?.status === 'error').length;
  const hasActivity = completed > 0 || running > 0 || errors > 0;

  return (
    <div style={{
      position: 'absolute',
      bottom: 0,
      left: 0,
      right: 0,
      height: 48,
      background: 'var(--bg-header)',
      borderTop: '1px solid var(--border)',
      display: 'flex',
      alignItems: 'center',
      gap: 20,
      padding: '0 18px',
      fontSize: 17,
      color: 'var(--text-dim)',
      zIndex: 5,
    }}>
      <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{
          width: 10,
          height: 10,
          borderRadius: '50%',
          background: connected ? 'var(--color-done)' : 'var(--color-error)',
          display: 'inline-block',
        }} />
        {connected ? 'Live' : 'Disconnected'}
      </span>

      {total > 0 && (
        <span style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          {/* Progress bar */}
          <span style={{
            width: 120,
            height: 6,
            borderRadius: 3,
            background: 'var(--border-sub)',
            overflow: 'hidden',
            display: 'inline-block',
          }}>
            <span style={{
              display: 'block',
              height: '100%',
              borderRadius: 3,
              width: hasActivity ? `${(completed / total) * 100}%` : '0%',
              background: errors > 0 ? 'var(--color-error)' : running > 0 ? 'var(--color-running)' : 'var(--color-done)',
              transition: 'width 0.3s ease',
            }} />
          </span>
          <span style={{ color: 'var(--text-muted)', fontSize: 14 }}>
            {hasActivity ? (
              <>
                {completed}/{total} completed
                {running > 0 && <span style={{ color: 'var(--color-running)' }}> · {running} running</span>}
                {errors > 0 && <span style={{ color: 'var(--color-error)' }}> · {errors} failed</span>}
              </>
            ) : (
              <span style={{ color: 'var(--text-dim)' }}>Idle · {total} nodes</span>
            )}
          </span>
        </span>
      )}

      {runCount > 0 && (
        <span style={{
          fontSize: 13,
          color: 'var(--text-dim)',
          background: 'var(--bg-tag)',
          border: '1px solid var(--border-tag)',
          padding: '2px 8px',
          borderRadius: 4,
        }}>
          {runCount} {runCount === 1 ? 'run' : 'runs'}
        </span>
      )}

      {lastEvent && (
        <span style={{ color: 'var(--text-dimmer)', marginLeft: 'auto', fontSize: 14 }}>
          {lastEvent.kind.replace(/_/g, ' ')}
          {' — '}
          <span style={{ color: 'var(--text-muted)' }}>{lastEvent.node_name}</span>
          {lastEvent.dataset_name && (
            <span style={{ color: 'var(--text-dimmer)' }}> / {lastEvent.dataset_name}</span>
          )}
          {lastEvent.duration_ms != null && (
            <span style={{ color: 'var(--color-done)' }}> {lastEvent.duration_ms.toFixed(1)}ms</span>
          )}
          {lastEvent.error && (
            <span style={{ color: 'var(--color-error)' }}> {lastEvent.error}</span>
          )}
        </span>
      )}
    </div>
  );
}
