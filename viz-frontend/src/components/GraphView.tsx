import { useCallback, useMemo } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  type NodeTypes,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import type { VizGraph, NodeStatus, DatasetActivity } from '../api/types';
import { useGraph } from '../hooks/useGraph';
import { LeafNode } from './nodes/LeafNode';
import { DatasetNode } from './nodes/DatasetNode';
import { PipelineNode } from './nodes/PipelineNode';

const nodeTypes: NodeTypes = {
  leaf: LeafNode as never,
  dataset: DatasetNode as never,
  pipeline: PipelineNode as never,
};

interface Props {
  graph: VizGraph | null;
  nodeStatuses: Record<string, NodeStatus>;
  datasetActivity: Record<string, DatasetActivity>;
  onDatasetSelect: (id: number) => void;
  onNodeSelect: (name: string) => void;
  isDark: boolean;
}

export function GraphView({ graph, nodeStatuses, datasetActivity, onDatasetSelect, onNodeSelect, isDark }: Props) {
  const stableOnDatasetSelect = useCallback(onDatasetSelect, [onDatasetSelect]);
  const stableOnNodeSelect = useCallback(onNodeSelect, [onNodeSelect]);
  const { nodes: layoutedNodes, edges: layoutedEdges } = useGraph(
    graph,
    nodeStatuses,
    datasetActivity,
    stableOnDatasetSelect,
    stableOnNodeSelect,
  );

  const [, , onNodesChange] = useNodesState(layoutedNodes);
  const [, , onEdgesChange] = useEdgesState(layoutedEdges);

  const syncedNodes = useMemo(() => layoutedNodes, [layoutedNodes]);
  const syncedEdges = useMemo(() => layoutedEdges, [layoutedEdges]);

  const minimapNodeColor = useCallback((node: { type?: string; data?: Record<string, unknown> }) => {
    if (node.type === 'dataset') {
      return node.data?.is_param
        ? (isDark ? '#6366f1' : '#4f46e5')
        : (isDark ? '#4ade80' : '#16a34a');
    }
    if (node.type === 'leaf') return isDark ? '#6a6a6a' : '#888888';
    return 'transparent';
  }, [isDark]);

  const minimapStyle = {
    background: isDark ? '#1a1a1a' : '#e8e8e8',
    border: `1px solid ${isDark ? '#333' : '#cccccc'}`,
    borderRadius: 6,
  };

  return (
    <div style={{ width: '100%', height: '100%', background: 'var(--bg-canvas)' }}>
      <ReactFlow
        nodes={syncedNodes}
        edges={syncedEdges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.2}
        colorMode={isDark ? 'dark' : 'light'}
      >
        <Background color="var(--grid-color)" gap={20} />
        <Controls />
        <MiniMap
          nodeColor={minimapNodeColor as never}
          nodeStrokeWidth={0}
          maskColor={isDark ? 'rgba(0,0,0,0.45)' : 'rgba(255,255,255,0.5)'}
          style={minimapStyle}
        />
      </ReactFlow>
    </div>
  );
}
