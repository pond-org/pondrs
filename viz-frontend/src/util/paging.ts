import type { VizNode, NodeStatus } from '../api/types';
import { stepStatus } from './status';

/**
 * Groups with more children than this expand into a window of consecutive
 * children (with the rest folded into "earlier"/"later" stubs) rather than
 * all at once.
 */
export const PAGE_THRESHOLD = 12;

/** Window sizes offered by the pager. */
export const PAGE_SIZES = [1, 3, 5] as const;

/** A paged group's window, as chosen by the user. */
export interface PageState {
  start: number;
  size: number;
  /** Keep the window on the child that is running (or last ran). */
  follow: boolean;
}

export const DEFAULT_PAGE: PageState = { start: 0, size: 1, follow: true };

export const isPaged = (node: VizNode) => node.is_pipe && node.pipe_children.length > PAGE_THRESHOLD;

/**
 * The child a following window should show: the first running child, else the
 * last child with any status (so a finished run stays on its final step).
 */
export function followTarget(
  group: VizNode,
  nodesById: Map<number, VizNode>,
  nodeStatuses: Record<string, NodeStatus>,
): number | null {
  let last: number | null = null;
  for (const [i, id] of group.pipe_children.entries()) {
    const child = nodesById.get(id);
    if (!child) continue;
    const s = stepStatus(child, nodesById, nodeStatuses);
    if (s === 'running') return i;
    if (s !== 'pending') last = i;
  }
  return last;
}

/** The window actually shown: `state`, moved onto the follow target when following. */
export function pageWindow(
  group: VizNode,
  state: PageState,
  nodesById: Map<number, VizNode>,
  nodeStatuses: Record<string, NodeStatus>,
): { start: number; size: number; target: number | null } {
  const n = group.pipe_children.length;
  const size = Math.max(1, Math.min(state.size, n));
  const target = followTarget(group, nodesById, nodeStatuses);
  let start = state.start;
  if (state.follow && target != null) start = target - Math.floor(size / 2);
  start = Math.max(0, Math.min(start, n - size));
  return { start, size, target };
}
