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
import type { GroupNodeData } from '../components/nodes/GroupNode';
import { leafProgress, groupStatus } from '../util/status';
import { DEFAULT_PAGE, isPaged, pageWindow, type PageState } from '../util/paging';
import { chain, commonBox } from '../util/grouping';
import type { ViewOptions } from '../util/view';
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
  /** The group box it is drawn in; undefined = top level. */
  box: string | undefined;
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
  view: ViewOptions,
): { nodes: Node[]; edges: Edge[]; pagers: PagerInfo[] } {
  // Only the fields used, so the memo does not depend on the whole object.
  const { theme, boxes, datasets: showDatasets, params: showParams } = view;
  return useMemo(() => {
    if (!graph) return { nodes: [], edges: [], pagers: [] };

    const markerColor = theme === 'dark' ? '#777' : '#999';

    const nodesById = new Map<number, VizNode>();
    for (const n of graph.nodes) nodesById.set(n.id, n);
    const datasetsById = new Map<number, VizDataset>();
    for (const ds of graph.datasets) datasetsById.set(ds.id, ds);

    const rfNodes: Node[] = [];
    const rfEdges: Edge[] = [];
    const steps: StepItem[] = [];
    const pagers: PagerInfo[] = [];
    // group box id → the box it sits in.
    const boxParent = new Map<string, string | undefined>();

    const emitStep = (node: VizNode, box: string | undefined) => {
      const rfId = `node-${node.id}`;
      steps.push({ rfId, inputs: node.input_dataset_ids, outputs: node.output_dataset_ids, box });
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
        rfNodes.push({ id: rfId, type: 'pipeline', position: { x: 0, y: 0 }, data, parentId: box });
      } else {
        const status = nodeStatuses[node.name];
        const data: LeafNodeData = {
          label: node.name,
          status: status?.status ?? 'pending',
          duration_ms: status?.duration_ms ?? null,
          error: status?.error ?? null,
          onSelect: () => onNodeSelect(node.name),
        };
        rfNodes.push({ id: rfId, type: 'leaf', position: { x: 0, y: 0 }, data, parentId: box });
      }
    };

    // The children of a paged group outside its window, folded into one step.
    const emitStub = (
      group: VizNode, side: 'before' | 'after', ids: number[], onClick: () => void, box: string | undefined,
    ) => {
      const hidden = ids.map(id => nodesById.get(id)).filter((n): n is VizNode => n != null);
      const rfId = `stub-${group.id}-${side}`;
      steps.push({ rfId, ...boundary(hidden), box });
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
      rfNodes.push({ id: rfId, type: 'stub', position: { x: 0, y: 0 }, data, parentId: box });
    };

    // Walk the step tree. Collapsed groups and leaves are drawn; expanded
    // groups dissolve into their children; paged groups show only a window
    // of their children, with stubs standing in for the rest. With `boxes`
    // on, an expanded group is drawn as a box around whatever it dissolved
    // into; `outer` is the box the current node sits in.
    const visit = (node: VizNode, outer: string | undefined) => {
      if (!node.is_pipe || !expandedPipelines.has(node.id)) {
        emitStep(node, outer);
        return;
      }
      const kids = node.pipe_children;
      let box = outer;
      if (boxes && kids.length > 0) {
        box = `group-${node.id}`;
        boxParent.set(box, outer);
        const pipeStatus = nodeStatuses[node.name];
        const progress = leafProgress(kids, nodesById, nodeStatuses);
        const data: GroupNodeData = {
          label: node.name,
          depth: chain(outer, boxParent).length,
          progress,
          status: groupStatus(pipeStatus?.status, progress),
          duration_ms: pipeStatus?.duration_ms ?? null,
          onToggle: () => onTogglePipeline(node.id),
        };
        rfNodes.push({
          id: box,
          type: 'box',
          position: { x: 0, y: 0 },
          data,
          parentId: outer,
          draggable: false,
          selectable: false,
          zIndex: -1,
        });
      }
      if (!isPaged(node)) {
        for (const id of kids) {
          const child = nodesById.get(id);
          if (child) visit(child, box);
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
          () => onPageChange(node.id, { start: Math.max(0, start - size), follow: false }), box);
      }
      for (const id of kids.slice(start, end)) {
        const child = nodesById.get(id);
        if (child) visit(child, box);
      }
      if (end < kids.length) {
        emitStub(node, 'after', kids.slice(end),
          () => onPageChange(node.id, { start: Math.min(kids.length - size, end), follow: false }), box);
      }
    };
    for (const node of graph.nodes) {
      if (node.parent_pipe == null) visit(node, undefined);
    }

    // Who touches each visible dataset. Datasets in the same sequence with the
    // same connections are interchangeable on the canvas, so they get one node.
    const producers = new Map<number, string[]>();
    const consumers = new Map<number, string[]>();
    for (const step of steps) {
      for (const id of step.outputs) producers.set(id, [...(producers.get(id) ?? []), step.rfId]);
      for (const id of step.inputs) consumers.set(id, [...(consumers.get(id) ?? []), step.rfId]);
    }

    // Hidden datasets. Params go entirely (nothing produces them, so nothing
    // needs reconnecting). A dataset passed between visible steps becomes an
    // edge from each producer to each consumer; sources and sinks stay, so the
    // pipeline's ends remain visible.
    const passed = (id: number) => producers.has(id) && consumers.has(id);
    const hidden = (id: number) => {
      const ds = datasetsById.get(id);
      if (!ds) return false;
      return ds.is_param ? !showParams : !showDatasets && passed(id);
    };
    const touchedIds = [...new Set([...producers.keys(), ...consumers.keys()])];
    const visibleIds = touchedIds.filter(id => !hidden(id));
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

    // A dataset (or bundle) lives where it is produced: the innermost box
    // holding every visible step that produces it. One nothing visible
    // produces (a source, a param) goes with its consumers instead. So a
    // box's outputs sit at its right edge and its edges leave it once, rather
    // than leaving and coming back when a consumer outside shares them.
    const stepBox = new Map(steps.map(s => [s.rfId, s.box]));
    const boxOf = (members: VizDataset[]): string | undefined => {
      if (!boxes) return undefined;
      const made = members.flatMap(m => producers.get(m.id) ?? []);
      const touching = made.length > 0 ? made : members.flatMap(m => consumers.get(m.id) ?? []);
      return commonBox(touching.map(rfId => chain(stepBox.get(rfId), boxParent)));
    };

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
      rfNodes.push({ id: rfId, type: 'bundle', position: { x: 0, y: 0 }, data, parentId: boxOf(members) });
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
        parentId: boxOf([ds]),
        // A dataset is "produced" if some visible step outputs it.
        style: producers.has(ds.id) ? {} : { opacity: 0.9 },
      });
    }

    // Edges, deduplicated: a bundle is joined to each step once.
    const edgeIds = new Set<string>();
    const pushEdge = (source: string, target: string, extra: Partial<Edge> = {}) => {
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
        ...extra,
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


    // Step-to-step edges standing in for hidden datasets: one per
    // (producer, consumer), labelled with what it carries. One carrying a
    // single dataset selects it when clicked.
    const carried = new Map<string, { source: string; target: string; carries: VizDataset[] }>();
    for (const id of touchedIds) {
      const ds = datasetsById.get(id);
      if (!ds || ds.is_param || !hidden(id)) continue;
      for (const source of producers.get(id) ?? []) {
        for (const target of consumers.get(id) ?? []) {
          if (source === target) continue;
          const key = `${source}>${target}`;
          const entry = carried.get(key) ?? { source, target, carries: [] };
          entry.carries.push(ds);
          carried.set(key, entry);
        }
      }
    }
    for (const { source, target, carries } of carried.values()) {
      const single = carries.length === 1 ? carries[0] : null;
      pushEdge(source, target, {
        label: single ? shortName(single.name) : `${carries.length} datasets`,
        labelStyle: single
          ? { fill: 'var(--color-dataset)', fontSize: 12, cursor: 'pointer' }
          : { fill: 'var(--text-muted)', fontSize: 12 },
        labelBgStyle: { fill: 'var(--bg-canvas)' },
        labelBgPadding: [4, 2],
        data: single ? { datasetId: single.id } : undefined,
        ...(single ? { style: { stroke: 'var(--edge-color)', strokeWidth: 2, cursor: 'pointer' } } : {}),
      });
    }

    const laid = layoutNodes(rfNodes, rfEdges);
    return { nodes: laid, edges: rfEdges, pagers };
  }, [
    graph, nodeStatuses, datasetActivity, onDatasetSelect, onNodeSelect, onTogglePipeline,
    expandedPipelines, pages, onPageChange, theme, boxes, showDatasets, showParams,
  ]);
}
