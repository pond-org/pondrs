import type { VizNode, NodeStatus, StatusKind } from '../api/types';
import type { GroupProgress } from '../components/nodes/PipelineNode';

/** Tally the execution status of every leaf beneath the given steps, recursively. */
export function leafProgress(
  ids: Iterable<number>,
  nodesById: Map<number, VizNode>,
  nodeStatuses: Record<string, NodeStatus>,
): GroupProgress {
  const p: GroupProgress = { total: 0, completed: 0, running: 0, error: 0 };
  const stack = [...ids];
  while (stack.length > 0) {
    const node = nodesById.get(stack.pop()!);
    if (!node) continue;
    if (node.is_pipe) {
      stack.push(...node.pipe_children);
      continue;
    }
    p.total += 1;
    const s = nodeStatuses[node.name]?.status;
    if (s === 'completed') p.completed += 1;
    else if (s === 'running') p.running += 1;
    else if (s === 'error') p.error += 1;
  }
  return p;
}

/** A group's status as reported by its own pipeline hooks, else derived from its leaves. */
export function groupStatus(own: StatusKind | undefined, p: GroupProgress): StatusKind {
  if (p.error > 0 || own === 'error') return 'error';
  if (own && own !== 'pending') return own;
  if (p.total > 0 && p.completed === p.total) return 'completed';
  if (p.running > 0 || p.completed > 0) return 'running';
  return 'pending';
}

/** Status of one step: a leaf's own, a group's derived from its leaves. */
export function stepStatus(
  node: VizNode,
  nodesById: Map<number, VizNode>,
  nodeStatuses: Record<string, NodeStatus>,
): StatusKind {
  const own = nodeStatuses[node.name]?.status;
  if (!node.is_pipe) return own ?? 'pending';
  return groupStatus(own, leafProgress(node.pipe_children, nodesById, nodeStatuses));
}
