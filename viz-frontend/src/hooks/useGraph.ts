import { useMemo } from 'react';
import { MarkerType } from '@xyflow/react';
import type { Node, Edge } from '@xyflow/react';
import type { VizGraph, VizNode, NodeStatus, DatasetActivity } from '../api/types';
import { layoutNodes } from '../layout/dagre';
import type { LeafNodeData } from '../components/nodes/LeafNode';
import type { DatasetNodeData } from '../components/nodes/DatasetNode';
import type { PipelineNodeData } from '../components/nodes/PipelineNode';

/** Check whether all ancestor pipelines of a node are expanded. */
function ancestorsExpanded(node: VizNode, nodesById: Map<number, VizNode>, expanded: Set<number>): boolean {
  let cur = node.parent_pipe;
  while (cur != null) {
    if (!expanded.has(cur)) return false;
    cur = nodesById.get(cur)?.parent_pipe ?? null;
  }
  return true;
}

export function useGraph(
  graph: VizGraph | null,
  nodeStatuses: Record<string, NodeStatus>,
  datasetActivity: Record<string, DatasetActivity>,
  onDatasetSelect: (id: number) => void,
  onNodeSelect: (name: string) => void,
  onTogglePipeline: (id: number) => void,
  expandedPipelines: Set<number>,
  isDark: boolean,
): { nodes: Node[]; edges: Edge[] } {
  return useMemo(() => {
    if (!graph) return { nodes: [], edges: [] };

    const markerColor = isDark ? '#777' : '#999';

    // Index nodes by id for fast lookup.
    const nodesById = new Map<number, VizNode>();
    for (const n of graph.nodes) nodesById.set(n.id, n);

    // Determine which nodes are visible.
    // A node is visible if all its ancestor pipelines are expanded,
    // AND it is not itself an expanded pipeline (those disappear).
    const visibleNodes: VizNode[] = [];
    for (const node of graph.nodes) {
      if (!ancestorsExpanded(node, nodesById, expandedPipelines)) continue;
      if (node.is_pipe && expandedPipelines.has(node.id)) continue;
      visibleNodes.push(node);
    }

    // Collect dataset ids referenced by visible nodes.
    const visibleDatasetIds = new Set<number>();
    for (const node of visibleNodes) {
      for (const id of node.input_dataset_ids) visibleDatasetIds.add(id);
      for (const id of node.output_dataset_ids) visibleDatasetIds.add(id);
    }

    const rfNodes: Node[] = [];
    const rfEdges: Edge[] = [];

    // Dataset nodes — only those referenced by visible nodes.
    const addedDatasets = new Set<number>();
    for (const ds of graph.datasets) {
      if (!visibleDatasetIds.has(ds.id)) continue;
      if (addedDatasets.has(ds.id)) continue;
      addedDatasets.add(ds.id);

      // A dataset is "produced" if some visible node outputs it.
      const isProduced = visibleNodes.some(n => n.output_dataset_ids.includes(ds.id));

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
        style: isProduced ? {} : { opacity: 0.9 },
      });
    }

    // Node nodes — visible leaf and collapsed pipeline nodes.
    for (const node of visibleNodes) {
      if (node.is_pipe) {
        // Collapsed pipeline node.
        const pipeStatus = nodeStatuses[node.name];
        const data: PipelineNodeData = {
          label: node.name,
          childCount: node.pipe_children.length,
          status: pipeStatus?.status ?? 'pending',
          duration_ms: pipeStatus?.duration_ms ?? null,
          onToggle: () => onTogglePipeline(node.id),
        };
        rfNodes.push({
          id: `node-${node.id}`,
          type: 'pipeline',
          position: { x: 0, y: 0 },
          data,
        });
      } else {
        // Leaf node.
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
        });
      }

      // Edges — same logic for both leaf and collapsed pipeline nodes.
      for (const dsId of node.input_dataset_ids) {
        if (!visibleDatasetIds.has(dsId)) continue;
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
        if (!visibleDatasetIds.has(dsId)) continue;
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
  }, [graph, nodeStatuses, datasetActivity, onDatasetSelect, onNodeSelect, onTogglePipeline, expandedPipelines, isDark]);
}
