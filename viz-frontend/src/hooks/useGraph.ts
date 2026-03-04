import { useMemo } from 'react';
import { MarkerType } from '@xyflow/react';
import type { Node, Edge } from '@xyflow/react';
import type { VizGraph, NodeStatus, DatasetActivity } from '../api/types';
import { layoutNodes } from '../layout/dagre';
import type { LeafNodeData } from '../components/nodes/LeafNode';
import type { DatasetNodeData } from '../components/nodes/DatasetNode';
import type { PipelineNodeData } from '../components/nodes/PipelineNode';

export function useGraph(
  graph: VizGraph | null,
  nodeStatuses: Record<string, NodeStatus>,
  datasetActivity: Record<string, DatasetActivity>,
  onDatasetSelect: (id: number) => void,
  onNodeSelect: (name: string) => void,
  isDark: boolean,
): { nodes: Node[]; edges: Edge[] } {
  return useMemo(() => {
    if (!graph) return { nodes: [], edges: [] };

    const markerColor = isDark ? '#777' : '#999';
    const rfNodes: Node[] = [];
    const rfEdges: Edge[] = [];

    const producedIds = new Set(graph.edges.map(e => e.dataset_id));

    const addedDatasets = new Set<number>();
    for (const ds of graph.datasets) {
      if (addedDatasets.has(ds.id)) continue;
      addedDatasets.add(ds.id);

      const activity = datasetActivity[ds.name] ?? null;
      const data: DatasetNodeData = {
        label: ds.name.replace(/^(catalog|params)\./, ''),
        is_param: ds.is_param,
        has_html: ds.has_html,
        dataset_id: ds.id,
        activity,
        onSelect: onDatasetSelect,
      };
      rfNodes.push({
        id: `ds-${ds.id}`,
        type: 'dataset',
        position: { x: 0, y: 0 },
        data,
        style: producedIds.has(ds.id) ? {} : { opacity: 0.9 },
      });
    }

    for (const node of graph.nodes) {
      const parentId = node.parent_pipe != null ? `node-${node.parent_pipe}` : undefined;

      if (node.is_pipe) {
        const data: PipelineNodeData = { label: node.name };
        rfNodes.push({
          id: `node-${node.id}`,
          type: 'pipeline',
          position: { x: 0, y: 0 },
          data,
          parentId,
          extent: parentId ? 'parent' : undefined,
          style: { width: 200, height: 100 },
        });
      } else {
        const status = nodeStatuses[node.name];
        const nodeName = node.name;
        const data: LeafNodeData = {
          label: nodeName,
          status: status?.status ?? 'pending',
          duration_ms: status?.duration_ms ?? null,
          error: status?.error ?? null,
          onSelect: () => onNodeSelect(nodeName),
        };
        rfNodes.push({
          id: `node-${node.id}`,
          type: 'leaf',
          position: { x: 0, y: 0 },
          data,
          parentId,
          extent: parentId ? 'parent' : undefined,
        });
      }
    }

    for (const node of graph.nodes) {
      if (node.is_pipe) continue;
      for (const dsId of node.input_dataset_ids) {
        rfEdges.push({
          id: `e-ds${dsId}-node${node.id}`,
          source: `ds-${dsId}`,
          target: `node-${node.id}`,
          type: 'smoothstep',
          style: { stroke: 'var(--edge-color)', strokeWidth: 2 },
          markerEnd: { type: MarkerType.ArrowClosed, color: markerColor, width: 10, height: 10 },
        });
      }
      for (const dsId of node.output_dataset_ids) {
        rfEdges.push({
          id: `e-node${node.id}-ds${dsId}`,
          source: `node-${node.id}`,
          target: `ds-${dsId}`,
          type: 'smoothstep',
          style: { stroke: 'var(--edge-color)', strokeWidth: 2 },
          markerEnd: { type: MarkerType.ArrowClosed, color: markerColor, width: 10, height: 10 },
        });
      }
    }

    const laid = layoutNodes(rfNodes, rfEdges);
    return { nodes: laid, edges: rfEdges };
  }, [graph, nodeStatuses, datasetActivity, onDatasetSelect, onNodeSelect, isDark]);
}
