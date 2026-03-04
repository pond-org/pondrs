import type { NodeProps, Node } from '@xyflow/react';

export type PipelineNodeData = {
  label: string;
};

export type PipelineNodeType = Node<PipelineNodeData, 'pipeline'>;

export function PipelineNode({ data }: NodeProps<PipelineNodeType>) {
  return (
    <div style={{
      background: 'rgba(255,255,255,0.03)',
      border: '1px dashed #444',
      borderRadius: 10,
      padding: '24px 12px 8px',
      minWidth: 200,
      minHeight: 80,
      color: '#666',
      fontSize: 11,
      pointerEvents: 'none',
    }}>
      <div style={{
        position: 'absolute',
        top: 6,
        left: 10,
        fontWeight: 600,
        letterSpacing: '0.05em',
        textTransform: 'uppercase',
        fontSize: 10,
        color: '#555',
      }}>
        {data.label}
      </div>
    </div>
  );
}
