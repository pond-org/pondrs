import type { NodeProps, Node } from '@xyflow/react';
import type { StatusKind } from '../../api/types';
import { ProgressBar, type GroupProgress } from './PipelineNode';
import { GROUP_HEADER_H } from '../../layout/dagre';

/** A box drawn around the children of an expanded group. */
export type GroupNodeData = {
  label: string;          // full path
  depth: number;          // for alternating tints
  progress: GroupProgress;
  status: StatusKind;
  duration_ms: number | null;
  onToggle: () => void;
};

export type GroupNodeType = Node<GroupNodeData, 'box'>;

const BORDER: Record<StatusKind, string> = {
  pending: 'var(--group-border)',
  running: 'var(--color-running)',
  completed: 'var(--color-done)',
  error: 'var(--color-error)',
};

function statusText(data: GroupNodeData): string {
  const { completed, total } = data.progress;
  switch (data.status) {
    case 'running': return `${completed}/${total} steps`;
    case 'completed': return data.duration_ms != null ? `✓ ${data.duration_ms.toFixed(1)}ms` : '✓ done';
    case 'error': return `error · ${completed}/${total}`;
    case 'pending': return `${total} ${total === 1 ? 'step' : 'steps'}`;
  }
}

export function GroupNode({ data }: NodeProps<GroupNodeType>) {
  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        background: data.depth % 2 === 0 ? 'var(--group-bg)' : 'var(--group-bg-alt)',
        border: `2px dashed ${BORDER[data.status]}`,
        borderRadius: 14,
        transition: 'border-color 0.3s',
        // Clicks on the body fall through to the pane and the children.
        pointerEvents: 'none',
        fontFamily: 'Inter, system-ui, sans-serif',
      }}
    >
      <div
        onClick={e => { e.stopPropagation(); data.onToggle(); }}
        title="Collapse"
        style={{
          pointerEvents: 'auto',
          cursor: 'pointer',
          height: GROUP_HEADER_H,
          padding: '8px 16px',
          display: 'inline-flex',
          flexDirection: 'column',
          justifyContent: 'center',
          minWidth: 220,
          maxWidth: '100%',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 12 }}>
          <span style={{ fontWeight: 600, fontSize: 20, color: 'var(--text)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
            {data.label}
          </span>
          <span style={{ fontSize: 14, color: 'var(--text-muted)', whiteSpace: 'nowrap' }}>{statusText(data)}</span>
          <span style={{ fontSize: 14, color: 'var(--text-dim)', whiteSpace: 'nowrap', marginLeft: 'auto' }}>collapse ↙</span>
        </div>
        <ProgressBar progress={data.progress} />
      </div>
    </div>
  );
}
