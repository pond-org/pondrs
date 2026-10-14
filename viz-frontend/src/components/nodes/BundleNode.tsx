import { Handle, Position } from '@xyflow/react';
import type { NodeProps, Node } from '@xyflow/react';

/** Several datasets with identical connections, drawn as one stacked node. */
export type BundleNodeData = {
  label: string;
  count: number;
  /** Members saved during the current run. */
  saved: number;
  /** Members loaded during the current run. */
  loaded: number;
  is_param: boolean;
  onSelect: () => void;
};

export type BundleNodeType = Node<BundleNodeData, 'bundle'>;

export function BundleNode({ data }: NodeProps<BundleNodeType>) {
  const isParam = data.is_param;
  const color = isParam ? 'var(--color-param)' : 'var(--color-dataset)';
  const style = isParam ? 'dashed' : 'solid';

  return (
    <div
      onClick={e => { e.stopPropagation(); data.onSelect(); }}
      title={`${data.count} datasets — click to preview the latest`}
      style={{
        background: isParam ? 'var(--bg-param)' : 'var(--bg-dataset)',
        border: `1.5px ${style} ${color}`,
        borderRadius: 24,
        padding: '6px 18px',
        minWidth: 120,
        maxWidth: 210,
        color: 'var(--text)',
        fontSize: 18,
        cursor: 'pointer',
        textAlign: 'center',
        fontFamily: 'Inter, system-ui, sans-serif',
        // A second outline offset behind the node reads as a stack.
        boxShadow: `4px 4px 0 -1.5px var(--bg-canvas), 4px 4px 0 0 ${color}`,
      }}
    >
      <Handle type="target" position={Position.Left} style={{ background: 'var(--handle-color)' }} />

      <div style={{ fontWeight: 500, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
        {data.label}
      </div>

      <div style={{ fontSize: 15, color: 'var(--text-dim)', marginTop: 2 }}>
        {data.saved > 0
          ? `↑ ${data.saved}/${data.count} saved`
          : data.loaded > 0
            ? `↓ ${data.loaded}/${data.count} loaded`
            : `×${data.count}`}
      </div>

      <Handle type="source" position={Position.Right} style={{ background: 'var(--handle-color)' }} />
    </div>
  );
}
