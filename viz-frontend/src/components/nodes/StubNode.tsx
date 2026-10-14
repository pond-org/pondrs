import { Handle, Position } from '@xyflow/react';
import type { NodeProps, Node } from '@xyflow/react';
import type { StatusKind } from '../../api/types';
import { ProgressBar, type GroupProgress } from './PipelineNode';

/** The children of a paged group that fall before or after its window. */
export type StubNodeData = {
  side: 'before' | 'after';
  count: number;
  /** Local names of the first and last hidden child. */
  first: string;
  last: string;
  progress: GroupProgress;
  status: StatusKind;
  /** Step the window one page towards this side. */
  onClick: () => void;
};

export type StubNodeType = Node<StubNodeData, 'stub'>;

const BORDER: Record<StatusKind, string> = {
  pending: 'var(--pipeline-border)',
  running: 'var(--color-running)',
  completed: 'var(--color-done)',
  error: 'var(--color-error)',
};

export function StubNode({ data }: NodeProps<StubNodeType>) {
  const before = data.side === 'before';
  const range = data.count === 1 ? data.first : `${data.first} … ${data.last}`;

  return (
    <div
      onClick={e => { e.stopPropagation(); data.onClick(); }}
      title={before ? 'Show the previous page' : 'Show the next page'}
      style={{
        background: 'var(--pipeline-bg)',
        border: `2px dashed ${BORDER[data.status]}`,
        borderRadius: 10,
        padding: '9px 18px',
        minWidth: 160,
        maxWidth: 220,
        color: 'var(--text-sub)',
        fontSize: 18,
        cursor: 'pointer',
        opacity: 0.85,
        fontFamily: 'Inter, system-ui, sans-serif',
      }}
    >
      <Handle type="target" position={Position.Left} style={{ background: 'var(--handle-color)' }} />

      <div style={{ fontWeight: 600, whiteSpace: 'nowrap' }}>
        {before ? `◀ ${data.count} earlier` : `${data.count} later ▶`}
      </div>
      <div style={{ fontSize: 14, color: 'var(--text-muted)', marginTop: 3, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
        {range}
      </div>

      <ProgressBar progress={data.progress} />

      <Handle type="source" position={Position.Right} style={{ background: 'var(--handle-color)' }} />
    </div>
  );
}
