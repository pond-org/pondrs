import { useCallback, useMemo, useEffect } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  useReactFlow,
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

export type CenterRequest = { id: string; tick: number };

interface Props {
  graph: VizGraph | null;
  nodeStatuses: Record<string, NodeStatus>;
  datasetActivity: Record<string, DatasetActivity>;
  onDatasetSelect: (id: number) => void;
  onNodeSelect: (name: string) => void;
  isDark: boolean;
  centerRequest: CenterRequest | null;
  onPaneClick: () => void;
}

// Rendered inside ReactFlow so it has access to useReactFlow().
function CenterController({ centerRequest }: { centerRequest: CenterRequest | null }) {
  const { setCenter, getNode } = useReactFlow();

  useEffect(() => {
    if (!centerRequest) return;
    const node = getNode(centerRequest.id);
    if (!node) return;
    const w = node.measured?.width ?? node.width ?? 180;
    const h = node.measured?.height ?? node.height ?? 50;
    setCenter(
      node.position.x + w / 2,
      node.position.y + h / 2,
      { zoom: 1.2, duration: 600 },
    );
  }, [centerRequest, getNode, setCenter]);

  return null;
}

export function GraphView({
  graph, nodeStatuses, datasetActivity,
  onDatasetSelect, onNodeSelect,
  isDark, centerRequest, onPaneClick,
}: Props) {
  const stableOnDatasetSelect = useCallback(onDatasetSelect, [onDatasetSelect]);
  const stableOnNodeSelect = useCallback(onNodeSelect, [onNodeSelect]);

  const { nodes: layoutedNodes, edges: layoutedEdges } = useGraph(
    graph, nodeStatuses, datasetActivity,
    stableOnDatasetSelect, stableOnNodeSelect,
    isDark,
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
        onPaneClick={onPaneClick}
      >
        <Background color="var(--grid-color)" gap={20} />
        <Controls />
        <MiniMap
          nodeColor={minimapNodeColor as never}
          nodeStrokeWidth={0}
          maskColor={isDark ? 'rgba(0,0,0,0.45)' : 'rgba(255,255,255,0.5)'}
          style={minimapStyle}
        />
        <CenterController centerRequest={centerRequest} />
      </ReactFlow>
    </div>
  );
}
