import type { VizEvent } from '../api/types';

interface Props {
  connected: boolean;
  lastEvent: VizEvent | null;
}

export function StatusBar({ connected, lastEvent }: Props) {
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

      {lastEvent && (
        <span style={{ color: 'var(--text-dimmer)' }}>
          {lastEvent.event_type.replace(/_/g, ' ')}
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
