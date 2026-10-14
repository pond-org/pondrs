import { useMemo } from 'react';
import { MarkerType } from '@xyflow/react';
import type { Node, Edge } from '@xyflow/react';
import type { VizGraph, VizNode, VizDataset, NodeStatus, DatasetActivity } from '../api/types';
import { layoutNodes } from '../layout/dagre';
import type { LeafNodeData } from '../components/nodes/LeafNode';
import type { DatasetNodeData } from '../components/nodes/DatasetNode';
import type { BundleNodeData } from '../components/nodes/BundleNode';
import type { PipelineNodeData } from '../components/nodes/PipelineNode';
import type { StubNodeData } from '../components/nodes/StubNode';
import { leafProgress, groupStatus } from '../util/status';
import { DEFAULT_PAGE, isPaged, pageWindow, type PageState } from '../util/paging';
import { compareNatural, localName, sequenceIndex, sequencePattern, shortName } from '../util/sequences';

/**
 * Datasets with no sequence pattern are still folded into one node once a
 * single connection (same producers, same consumers) carries more than this.
 */
const OVERFLOW_THRESHOLD = 8;

/** An expanded, paged group as the pager shows it. */
export interface PagerInfo {
  groupId: number;
  name: string;
  /** Local names of the group's children, in order. */
  children: string[];
  start: number;
  size: number;
  follow: boolean;
  /** Index of the child a following window would show, if any has run. */
  target: number | null;
}

/** Something drawn as a step in the graph: a leaf, a collapsed group, or a stub. */
interface StepItem {
  rfId: string;
  inputs: number[];
  outputs: number[];
}

/** Datasets consumed but not produced by `nodes`, and the mirror — the rule `Unrolled` uses. */
function boundary(nodes: VizNode[]): { inputs: number[]; outputs: number[] } {
  const ins = new Set(nodes.flatMap(n => n.input_dataset_ids));
  const outs = new Set(nodes.flatMap(n => n.output_dataset_ids));
  return {
    inputs: [...ins].filter(id => !outs.has(id)),
    outputs: [...outs].filter(id => !ins.has(id)),
  };
}

/** The index range a sequence bundle spans: `0‥99`. */
function indexRange(members: VizDataset[]): string | null {
  const idx = members.map(m => sequenceIndex(m.name)).filter((i): i is number => i != null);
  return idx.length > 0 ? `${Math.min(...idx)}‥${Math.max(...idx)}` : null;
}

export function useGraph(
  graph: VizGraph | null,
  nodeStatuses: Record<string, NodeStatus>,
  datasetActivity: Record<string, DatasetActivity>,
  onDatasetSelect: (id: number) => void,
  onNodeSelect: (name: string) => void,
  onTogglePipeline: (id: number) => void,
  expandedPipelines: Set<number>,
  pages: Map<number, PageState>,
  onPageChange: (groupId: number, patch: Partial<PageState>) => void,
  isDark: boolean,
): { nodes: Node[]; edges: Edge[]; pagers: PagerInfo[] } {
  return useMemo(() => {
    if (!graph) return { nodes: [], edges: [], pagers: [] };

    const markerColor = isDark ? '#777' : '#999';

    const nodesById = new Map<number, VizNode>();
    for (const n of graph.nodes) nodesById.set(n.id, n);
    const datasetsById = new Map<number, VizDataset>();
    for (const ds of graph.datasets) datasetsById.set(ds.id, ds);

    const rfNodes: Node[] = [];
    const rfEdges: Edge[] = [];
    const steps: StepItem[] = [];
    const pagers: PagerInfo[] = [];

    const emitStep = (node: VizNode) => {
      const rfId = `node-${node.id}`;
      steps.push({ rfId, inputs: node.input_dataset_ids, outputs: node.output_dataset_ids });
      if (node.is_pipe) {
        const pipeStatus = nodeStatuses[node.name];
        const progress = leafProgress(node.pipe_children, nodesById, nodeStatuses);
        const data: PipelineNodeData = {
          label: node.name,
          childCount: node.pipe_children.length,
          progress,
          status: groupStatus(pipeStatus?.status, progress),
          duration_ms: pipeStatus?.duration_ms ?? null,
          onToggle: () => onTogglePipeline(node.id),
        };
        rfNodes.push({ id: rfId, type: 'pipeline', position: { x: 0, y: 0 }, data });
      } else {
        const status = nodeStatuses[node.name];
        const data: LeafNodeData = {
          label: node.name,
          status: status?.status ?? 'pending',
          duration_ms: status?.duration_ms ?? null,
          error: status?.error ?? null,
          onSelect: () => onNodeSelect(node.name),
        };
        rfNodes.push({ id: rfId, type: 'leaf', position: { x: 0, y: 0 }, data });
      }
    };

    // The children of a paged group outside its window, folded into one step.
    const emitStub = (group: VizNode, side: 'before' | 'after', ids: number[], onClick: () => void) => {
      const hidden = ids.map(id => nodesById.get(id)).filter((n): n is VizNode => n != null);
      const rfId = `stub-${group.id}-${side}`;
      steps.push({ rfId, ...boundary(hidden) });
      const progress = leafProgress(ids, nodesById, nodeStatuses);
      const data: StubNodeData = {
        side,
        count: hidden.length,
        first: localName(hidden[0]?.name ?? ''),
        last: localName(hidden[hidden.length - 1]?.name ?? ''),
        progress,
        status: groupStatus(undefined, progress),
        onClick,
      };
      rfNodes.push({ id: rfId, type: 'stub', position: { x: 0, y: 0 }, data });
    };

    // Walk the step tree. Collapsed groups and leaves are drawn; expanded
    // groups dissolve into their children; paged groups show only a window
    // of their children, with stubs standing in for the rest.
    const visit = (node: VizNode) => {
      if (!node.is_pipe || !expandedPipelines.has(node.id)) {
        emitStep(node);
        return;
      }
      const kids = node.pipe_children;
      if (!isPaged(node)) {
        for (const id of kids) {
          const child = nodesById.get(id);
          if (child) visit(child);
        }
        return;
      }
      const state = pages.get(node.id) ?? DEFAULT_PAGE;
      const { start, size, target } = pageWindow(node, state, nodesById, nodeStatuses);
      pagers.push({
        groupId: node.id,
        name: node.name,
        children: kids.map(id => localName(nodesById.get(id)?.name ?? '')),
        start, size, target,
        follow: state.follow,
      });
      const end = start + size;
      if (start > 0) {
        emitStub(node, 'before', kids.slice(0, start),
          () => onPageChange(node.id, { start: Math.max(0, start - size), follow: false }));
      }
      for (const id of kids.slice(start, end)) {
        const child = nodesById.get(id);
        if (child) visit(child);
      }
      if (end < kids.length) {
        emitStub(node, 'after', kids.slice(end),
          () => onPageChange(node.id, { start: Math.min(kids.length - size, end), follow: false }));
      }
    };
    for (const node of graph.nodes) {
      if (node.parent_pipe == null) visit(node);
    }

    // Who touches each visible dataset. Datasets in the same sequence with the
    // same connections are interchangeable on the canvas, so they get one node.
    const producers = new Map<number, string[]>();
    const consumers = new Map<number, string[]>();
    for (const step of steps) {
      for (const id of step.outputs) producers.set(id, [...(producers.get(id) ?? []), step.rfId]);
      for (const id of step.inputs) consumers.set(id, [...(consumers.get(id) ?? []), step.rfId]);
    }
    const visibleIds = [...new Set([...producers.keys(), ...consumers.keys()])];
    const signature = (id: number) =>
      `${[...(producers.get(id) ?? [])].sort().join(',')}>${[...(consumers.get(id) ?? [])].sort().join(',')}`;

    const bySequence = new Map<string, VizDataset[]>();
    const loose = new Map<string, VizDataset[]>();
    for (const id of visibleIds) {
      const ds = datasetsById.get(id);
      if (!ds) continue;
      const pattern = sequencePattern(ds.name);
      const sig = signature(id);
      const [map, key] = pattern != null ? [bySequence, `${pattern}|${sig}`] : [loose, sig];
      map.set(key, [...(map.get(key) ?? []), ds]);
    }

    // dataset id → id of the canvas node that shows it.
    const shownAs = new Map<number, string>();
    const singles: VizDataset[] = [];

    const emitBundle = (label: string, range: string | null, members: VizDataset[]) => {
      members.sort((a, b) => compareNatural(a.name, b.name));
      const rfId = `bundle-${members[0].id}`;
      for (const m of members) shownAs.set(m.id, rfId);
      const activity = members.map(m => datasetActivity[m.name]);
      // Preview the latest member that has seen activity, else the first.
      const latest = [...members].reverse().find(m => datasetActivity[m.name]) ?? members[0];
      const data: BundleNodeData = {
        label,
        range,
        count: members.length,
        saved: activity.filter(a => a?.save_ms != null).length,
        loaded: activity.filter(a => a?.load_ms != null).length,
        is_param: members.every(m => m.is_param),
        onSelect: () => onDatasetSelect(latest.id),
      };
      rfNodes.push({ id: rfId, type: 'bundle', position: { x: 0, y: 0 }, data });
    };

    for (const [key, members] of bySequence) {
      if (members.length >= 2) emitBundle(key.slice(0, key.indexOf('|')), indexRange(members), members);
      else singles.push(...members);
    }
    for (const members of loose.values()) {
      if (members.length > OVERFLOW_THRESHOLD) emitBundle(`${members.length} datasets`, null, members);
      else singles.push(...members);
    }

    for (const ds of singles) {
      const rfId = `ds-${ds.id}`;
      shownAs.set(ds.id, rfId);
      const data: DatasetNodeData = {
        label: shortName(ds.name),
        is_param: ds.is_param,
        has_html: ds.has_html,
        dataset_id: ds.id,
        activity: datasetActivity[ds.name] ?? null,
        onSelect: onDatasetSelect,
      };
      rfNodes.push({
        id: rfId,
        type: 'dataset',
        position: { x: 0, y: 0 },
        data,
        // A dataset is "produced" if some visible step outputs it.
        style: producers.has(ds.id) ? {} : { opacity: 0.9 },
      });
    }

    // Edges, deduplicated: a bundle is joined to each step once.
    const edgeIds = new Set<string>();
    const pushEdge = (source: string, target: string) => {
      const id = `e-${source}-${target}`;
      if (edgeIds.has(id)) return;
      edgeIds.add(id);
      rfEdges.push({
        id,
        source,
        target,
        type: 'smoothstep',
        style: { stroke: 'var(--edge-color)', strokeWidth: 2 },
        markerEnd: { type: MarkerType.ArrowClosed, color: markerColor, width: 10, height: 10 },
      });
    };
    for (const step of steps) {
      for (const id of step.inputs) {
        const ds = shownAs.get(id);
        if (ds) pushEdge(ds, step.rfId);
      }
      for (const id of step.outputs) {
        const ds = shownAs.get(id);
        if (ds) pushEdge(step.rfId, ds);
      }
    }

    const laid = layoutNodes(rfNodes, rfEdges);
    return { nodes: laid, edges: rfEdges, pagers };
  }, [
    graph, nodeStatuses, datasetActivity, onDatasetSelect, onNodeSelect, onTogglePipeline,
    expandedPipelines, pages, onPageChange, isDark,
  ]);
}
