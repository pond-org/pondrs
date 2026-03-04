import { Handle, Position } from '@xyflow/react';
import type { NodeProps, Node } from '@xyflow/react';
import type { StatusKind } from '../../api/types';

export type LeafNodeData = {
  label: string;
  status: StatusKind;
  duration_ms: number | null;
  error: string | null;
  onSelect: () => void;
};

export type LeafNodeType = Node<LeafNodeData, 'leaf'>;

const BORDER: Record<StatusKind, string> = {
  pending: '#4a4a4a',
  running: '#3b82f6',
  completed: '#22c55e',
  error: '#ef4444',
};

const BG: Record<StatusKind, string> = {
  pending: '#1e1e1e',
  running: '#1e2a3a',
  completed: '#162716',
  error: '#2a1616',
};

export function LeafNode({ data }: NodeProps<LeafNodeType>) {
  const border = BORDER[data.status];
  const bg = BG[data.status];

  return (
    <div
      onClick={e => { e.stopPropagation(); data.onSelect(); }}
      style={{
        background: bg,
        border: `2px solid ${border}`,
        borderRadius: 8,
        padding: '6px 12px',
        minWidth: 140,
        maxWidth: 180,
        color: '#e5e5e5',
        fontSize: 13,
        cursor: 'pointer',
        boxShadow: data.status === 'running' ? `0 0 8px ${border}66` : 'none',
        transition: 'border-color 0.3s, background 0.3s, box-shadow 0.3s',
      }}
    >
      <Handle type="target" position={Position.Left} style={{ background: '#555' }} />

      <div style={{ fontWeight: 600, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
        {data.label}
      </div>

      {data.status !== 'pending' && (
        <div style={{ fontSize: 11, color: '#888', marginTop: 2 }}>
          {data.status === 'running' && '⏳ running…'}
          {data.status === 'completed' && data.duration_ms != null && `✓ ${data.duration_ms.toFixed(1)}ms`}
          {data.status === 'completed' && data.duration_ms == null && '✓ done'}
          {data.status === 'error' && `✗ ${data.error ?? 'error'}`}
        </div>
      )}

      <Handle type="source" position={Position.Right} style={{ background: '#555' }} />
    </div>
  );
}
