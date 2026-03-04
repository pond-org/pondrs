import { Handle, Position } from '@xyflow/react';
import type { NodeProps, Node } from '@xyflow/react';
import type { DatasetActivity } from '../../api/types';

export type DatasetNodeData = {
  label: string;
  is_param: boolean;
  has_html: boolean;
  dataset_id: number;
  activity: DatasetActivity | null;
  onSelect: (id: number) => void;
};

export type DatasetNodeType = Node<DatasetNodeData, 'dataset'>;

export function DatasetNode({ data }: NodeProps<DatasetNodeType>) {
  const isParam = data.is_param;

  return (
    <div
      onClick={e => { e.stopPropagation(); data.onSelect(data.dataset_id); }}
      style={{
        background: isParam ? 'var(--bg-param)' : 'var(--bg-dataset)',
        border: `1.5px ${isParam ? 'dashed' : 'solid'} ${isParam ? 'var(--color-param)' : 'var(--color-dataset)'}`,
        borderRadius: 24,
        padding: '6px 18px',
        minWidth: 120,
        maxWidth: 170,
        color: 'var(--text)',
        fontSize: 18,
        cursor: 'pointer',
        textAlign: 'center',
        transition: 'opacity 0.2s',
        fontFamily: 'Inter, system-ui, sans-serif',
      }}
      title={data.has_html ? 'Click to preview' : undefined}
    >
      <Handle type="target" position={Position.Left} style={{ background: 'var(--handle-color)' }} />

      <div style={{ fontWeight: 500, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
        {data.label}
      </div>

      {data.activity && (
        <div style={{ fontSize: 15, color: 'var(--text-dim)', marginTop: 2 }}>
          {data.activity.load_ms != null && `↓${data.activity.load_ms.toFixed(0)}ms`}
          {data.activity.load_ms != null && data.activity.save_ms != null && ' '}
          {data.activity.save_ms != null && `↑${data.activity.save_ms.toFixed(0)}ms`}
        </div>
      )}

      {data.has_html && (
        <div style={{ fontSize: 15, color: 'var(--text-dim)', marginTop: 2 }}>preview ↗</div>
      )}

      <Handle type="source" position={Position.Right} style={{ background: 'var(--handle-color)' }} />
    </div>
  );
}
