import type { NodeProps, Node } from '@xyflow/react';

export type PipelineNodeData = {
  label: string;
};

export type PipelineNodeType = Node<PipelineNodeData, 'pipeline'>;

export function PipelineNode({ data }: NodeProps<PipelineNodeType>) {
  return (
    <div style={{
      background: 'var(--pipeline-bg)',
      border: '1px dashed var(--pipeline-border)',
      borderRadius: 12,
      padding: '30px 16px 10px',
      minWidth: 200,
      minHeight: 80,
      color: 'var(--pipeline-text)',
      fontSize: 16,
      pointerEvents: 'none',
      fontFamily: 'Inter, system-ui, sans-serif',
    }}>
      <div style={{
        position: 'absolute',
        top: 8,
        left: 12,
        fontWeight: 600,
        letterSpacing: '0.06em',
        textTransform: 'uppercase',
        fontSize: 15,
        color: 'var(--pipeline-text)',
      }}>
        {data.label}
      </div>
    </div>
  );
}
