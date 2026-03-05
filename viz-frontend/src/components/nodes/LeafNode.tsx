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
  pending: 'var(--border-node)',
  running: 'var(--color-running)',
  completed: 'var(--color-done)',
  error: 'var(--color-error)',
};

const BG: Record<StatusKind, string> = {
  pending: 'var(--bg-node)',
  running: 'var(--bg-node-run)',
  completed: 'var(--bg-node-done)',
  error: 'var(--bg-node-err)',
};

// For box-shadow we need concrete color values, so we use a separate map
const SHADOW_COLOR: Record<StatusKind, string> = {
  pending: 'transparent',
  running: '#3b82f666',
  completed: 'transparent',
  error: 'transparent',
};

export function LeafNode({ data }: NodeProps<LeafNodeType>) {
  const border = BORDER[data.status];
  const bg = BG[data.status];
  const shadow = SHADOW_COLOR[data.status];

  return (
    <div
      onClick={e => { e.stopPropagation(); data.onSelect(); }}
      style={{
        background: bg,
        border: `2px solid ${border}`,
        borderRadius: 10,
        padding: '9px 18px',
        minWidth: 160,
        maxWidth: 220,
        color: 'var(--text)',
        fontSize: 20,
        cursor: 'pointer',
        boxShadow: `0 0 12px ${shadow}`,
        transition: 'border-color 0.3s, background 0.3s, box-shadow 0.3s',
        fontFamily: 'Inter, system-ui, sans-serif',
      }}
    >
      <Handle type="target" position={Position.Left} style={{ background: 'var(--handle-color)' }} />

      <div style={{ fontWeight: 600, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
        {data.label}
      </div>

      {data.status !== 'pending' && (
        <div style={{ fontSize: 16, color: 'var(--text-muted)', marginTop: 3 }}>
          {data.status === 'running' && '⏳ running…'}
          {data.status === 'completed' && data.duration_ms != null && `✓ ${data.duration_ms.toFixed(1)}ms`}
          {data.status === 'completed' && data.duration_ms == null && '✓ done'}
          {data.status === 'error' && 'error ↗'}
        </div>
      )}

      <Handle type="source" position={Position.Right} style={{ background: 'var(--handle-color)' }} />
    </div>
  );
}
