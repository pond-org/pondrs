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
      height: 32,
      background: '#111',
      borderTop: '1px solid #222',
      display: 'flex',
      alignItems: 'center',
      gap: 16,
      padding: '0 14px',
      fontSize: 12,
      color: '#666',
      zIndex: 5,
    }}>
      <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <span style={{
          width: 8,
          height: 8,
          borderRadius: '50%',
          background: connected ? '#22c55e' : '#ef4444',
          display: 'inline-block',
        }} />
        {connected ? 'Live' : 'Disconnected'}
      </span>

      {lastEvent && (
        <span style={{ color: '#555' }}>
          {lastEvent.event_type.replace(/_/g, ' ')}
          {' — '}
          <span style={{ color: '#888' }}>{lastEvent.node_name}</span>
          {lastEvent.dataset_name && (
            <span style={{ color: '#555' }}> / {lastEvent.dataset_name}</span>
          )}
          {lastEvent.duration_ms != null && (
            <span style={{ color: '#4ade80' }}> {lastEvent.duration_ms.toFixed(1)}ms</span>
          )}
          {lastEvent.error && (
            <span style={{ color: '#ef4444' }}> {lastEvent.error}</span>
          )}
        </span>
      )}
    </div>
  );
}
