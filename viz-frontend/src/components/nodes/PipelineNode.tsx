import { Handle, Position } from '@xyflow/react';
import type { NodeProps, Node } from '@xyflow/react';
import type { StatusKind } from '../../api/types';

export type PipelineNodeData = {
  label: string;
  childCount: number;
  status: StatusKind;
  duration_ms: number | null;
  onToggle: () => void;
};

export type PipelineNodeType = Node<PipelineNodeData, 'pipeline'>;

const BORDER: Record<StatusKind, string> = {
  pending: 'var(--pipeline-border)',
  running: 'var(--color-running)',
  completed: 'var(--color-done)',
  error: 'var(--color-error)',
};

const BG: Record<StatusKind, string> = {
  pending: 'var(--pipeline-bg)',
  running: 'var(--bg-node-run)',
  completed: 'var(--bg-node-done)',
  error: 'var(--bg-node-err)',
};

const SHADOW_COLOR: Record<StatusKind, string> = {
  pending: 'transparent',
  running: '#3b82f666',
  completed: 'transparent',
  error: 'transparent',
};

export function PipelineNode({ data }: NodeProps<PipelineNodeType>) {
  const border = BORDER[data.status];
  const bg = BG[data.status];
  const shadow = SHADOW_COLOR[data.status];

  return (
    <div
      onClick={e => { e.stopPropagation(); data.onToggle(); }}
      style={{
        background: bg,
        border: `3px double ${border}`,
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

      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginTop: 3 }}>
        <span style={{ fontSize: 14, color: 'var(--text-muted)' }}>
          {data.status === 'running' && 'running…'}
          {data.status === 'completed' && data.duration_ms != null && `✓ ${data.duration_ms.toFixed(1)}ms`}
          {data.status === 'completed' && data.duration_ms == null && '✓ done'}
          {data.status === 'error' && 'error ↗'}
          {data.status === 'pending' && `${data.childCount} ${data.childCount === 1 ? 'step' : 'steps'}`}
        </span>
        <span style={{ fontSize: 14, color: 'var(--text-dim)' }}>expand ↗</span>
      </div>

      <Handle type="source" position={Position.Right} style={{ background: 'var(--handle-color)' }} />
    </div>
  );
}
