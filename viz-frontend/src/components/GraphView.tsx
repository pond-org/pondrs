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
}

export function GraphView({ graph, nodeStatuses, datasetActivity, onDatasetSelect, onNodeSelect }: Props) {
  const stableOnDatasetSelect = useCallback(onDatasetSelect, [onDatasetSelect]);
  const stableOnNodeSelect = useCallback(onNodeSelect, [onNodeSelect]);
  const { nodes: layoutedNodes, edges: layoutedEdges } = useGraph(
    graph,
    nodeStatuses,
    datasetActivity,
    stableOnDatasetSelect,
    stableOnNodeSelect,
  );

  const [nodes, , onNodesChange] = useNodesState(layoutedNodes);
  const [edges, , onEdgesChange] = useEdgesState(layoutedEdges);

  // Sync layout changes from memo into RF state.
  const syncedNodes = useMemo(() => layoutedNodes, [layoutedNodes]);
  const syncedEdges = useMemo(() => layoutedEdges, [layoutedEdges]);

  return (
    <div style={{ width: '100%', height: '100%', background: '#0f0f0f' }}>
      <ReactFlow
        nodes={syncedNodes}
        edges={syncedEdges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.2}
        colorMode="dark"
      >
        <Background color="#222" gap={20} />
        <Controls />
        <MiniMap nodeColor={() => '#333'} maskColor="rgba(0,0,0,0.6)" />
      </ReactFlow>
    </div>
  );
}
