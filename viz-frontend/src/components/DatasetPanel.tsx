import { useEffect, useState } from 'react';
import { fetchDatasetHtml } from '../api/client';
import type { NodeStatus, DatasetActivity } from '../api/types';

export type PanelSelection =
  | { kind: 'dataset'; id: number; name: string; is_param: boolean; activity: DatasetActivity | null }
  | { kind: 'node'; name: string; status: NodeStatus | null };

interface Props {
  selection: PanelSelection | null;
  onClose: () => void;
}

const STATUS_COLOR: Record<string, string> = {
  pending: '#666',
  running: '#3b82f6',
  completed: '#22c55e',
  error: '#ef4444',
};

export function DatasetPanel({ selection, onClose }: Props) {
  const [html, setHtml] = useState('');
  const [loading, setLoading] = useState(false);

  const datasetId = selection?.kind === 'dataset' ? selection.id : null;

  useEffect(() => {
    if (datasetId == null) { setHtml(''); return; }
    setLoading(true);
    fetchDatasetHtml(datasetId)
      .then(h => { setHtml(h); setLoading(false); })
      .catch(() => { setHtml(''); setLoading(false); });
  }, [datasetId]);

  const open = selection != null;

  return (
    <div style={{
      position: 'absolute',
      top: 0,
      right: 0,
      width: 480,
      height: '100%',
      background: '#141414',
      borderLeft: '1px solid #2a2a2a',
      display: 'flex',
      flexDirection: 'column',
      zIndex: 10,
      transform: open ? 'translateX(0)' : 'translateX(100%)',
      transition: 'transform 0.2s ease',
      pointerEvents: open ? 'auto' : 'none',
    }}>
      {/* Header */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '10px 14px',
        borderBottom: '1px solid #2a2a2a',
        gap: 8,
        flexShrink: 0,
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, overflow: 'hidden' }}>
          {selection && (
            <span style={{
              fontSize: 11,
              color: '#888',
              background: '#222',
              padding: '2px 7px',
              borderRadius: 4,
              flexShrink: 0,
              border: '1px solid #333',
            }}>
              {selection.kind === 'node' ? 'node' : selection.is_param ? 'param' : 'dataset'}
            </span>
          )}
          <span style={{
            color: '#ccc',
            fontSize: 13,
            fontWeight: 600,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}>
            {selection?.name.replace(/^(catalog|params)\./, '') ?? ''}
          </span>
        </div>
        <button
          onClick={onClose}
          style={{ background: 'none', border: 'none', color: '#888', cursor: 'pointer', fontSize: 18, lineHeight: 1, flexShrink: 0 }}
        >
          ×
        </button>
      </div>

      {/* Node body */}
      {selection?.kind === 'node' && (
        <div style={{ padding: 16, overflowY: 'auto' }}>
          <NodeInfo status={selection.status} />
        </div>
      )}

      {/* Dataset body */}
      {selection?.kind === 'dataset' && (
        <>
          {selection.activity && (
            <div style={{ padding: '8px 14px', borderBottom: '1px solid #1e1e1e', display: 'flex', gap: 16, flexShrink: 0 }}>
              {selection.activity.load_ms != null && (
                <span style={{ fontSize: 12, color: '#888' }}>↓ load: {selection.activity.load_ms.toFixed(1)}ms</span>
              )}
              {selection.activity.save_ms != null && (
                <span style={{ fontSize: 12, color: '#888' }}>↑ save: {selection.activity.save_ms.toFixed(1)}ms</span>
              )}
            </div>
          )}
          <div style={{ flex: 1, overflow: 'hidden' }}>
            {loading && (
              <div style={{ color: '#555', fontSize: 13, padding: 16 }}>Loading…</div>
            )}
            {!loading && !html && (
              <div style={{ color: '#555', fontSize: 13, padding: 16 }}>No preview available.</div>
            )}
            {!loading && html && (
              <iframe
                srcDoc={html}
                sandbox="allow-scripts allow-same-origin"
                style={{ width: '100%', height: '100%', border: 'none', background: '#fff' }}
                title="dataset preview"
              />
            )}
          </div>
        </>
      )}
    </div>
  );
}

function NodeInfo({ status }: { status: NodeStatus | null }) {
  if (!status || status.status === 'pending') {
    return <div style={{ color: '#555', fontSize: 13 }}>No execution data yet.</div>;
  }

  const color = STATUS_COLOR[status.status] ?? '#666';
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{
          fontSize: 12,
          color,
          background: `${color}22`,
          border: `1px solid ${color}44`,
          padding: '2px 8px',
          borderRadius: 4,
          fontWeight: 600,
        }}>
          {status.status}
        </span>
        {status.duration_ms != null && (
          <span style={{ fontSize: 12, color: '#888' }}>{status.duration_ms.toFixed(1)}ms</span>
        )}
      </div>
      {status.error && (
        <div style={{
          fontSize: 12,
          color: '#ef4444',
          background: '#2a1616',
          border: '1px solid #442222',
          borderRadius: 4,
          padding: '8px 10px',
          wordBreak: 'break-word',
        }}>
          {status.error}
        </div>
      )}
    </div>
  );
}
