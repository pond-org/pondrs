import Dagre from '@dagrejs/dagre';
import type { Node, Edge } from '@xyflow/react';

const NODE_W = 200;
const NODE_H = 70;
const PIPE_H = 84; // collapsed groups and stubs carry a progress bar
const DS_W = 160;
const DS_H = 56;
const BUNDLE_W = 210; // sequence labels carry an index range
const BUNDLE_H = 64;

/** Room dagre reserves for an edge label, per character and in height. */
const EDGE_LABEL_CHAR_W = 7;
const EDGE_LABEL_H = 18;

/** Room above a group box's children for its header. */
export const GROUP_HEADER_H = 64;
/** Gap between a group box's border and its children. */
const GROUP_PAD = 24;
/** Narrowest box, so the header's path, status and "collapse" fit. */
const GROUP_MIN_W = 300;

/** Stand-ins, inside a box's own layout, for everything outside it. */
const IN = '\0in';
const OUT = '\0out';

function sizeOf(node: Node): { w: number; h: number } {
  switch (node.type) {
    case 'dataset': return { w: DS_W, h: DS_H };
    case 'bundle': return { w: BUNDLE_W, h: BUNDLE_H };
    case 'pipeline':
    case 'stub': return { w: NODE_W, h: PIPE_H };
    default: return { w: NODE_W, h: NODE_H };
  }
}

/**
 * Lay out the canvas. Nodes of type `box` are group boxes around the nodes
 * whose `parentId` names them. Each box is laid out as a block, innermost
 * first: its children get their own dagre layout, which fixes the box's size,
 * and the enclosing layout then places the box as a single node of that size,
 * with every edge into or out of the box attached to it. So a box is only as
 * big as its contents, and whatever feeds it lands to its left and whatever
 * reads from it to its right. Inside a box, edges from outside attach to an
 * `IN` node and edges leaving to an `OUT` node, so entry members sit at the
 * box's left edge and exit members at its right.
 *
 * The result has children positioned relative to their box and parents before
 * children, as React Flow requires. Without boxes this is one plain dagre
 * layout.
 */
export function layoutNodes(nodes: Node[], edges: Edge[]): Node[] {
  const byId = new Map(nodes.map(n => [n.id, n]));
  const childrenOf = new Map<string | undefined, Node[]>();
  for (const node of nodes) {
    const key = node.parentId && byId.has(node.parentId) ? node.parentId : undefined;
    childrenOf.set(key, [...(childrenOf.get(key) ?? []), node]);
  }
  // The child of `container` that holds `id` (itself, or a box around it), or
  // undefined when `id` is outside `container`.
  const childIn = (id: string, container: string | undefined): string | undefined => {
    for (let cur: string | undefined = id; cur !== undefined; cur = byId.get(cur)?.parentId) {
      if (byId.get(cur)?.parentId === container) return cur;
    }
    return undefined;
  };

  const size = new Map<string, { w: number; h: number }>();
  // Top-left corner, relative to the enclosing box (absolute at the top level).
  const position = new Map<string, { x: number; y: number }>();

  // Lay out the children of `container`; returns the size they take up.
  const layoutLevel = (container: string | undefined): { w: number; h: number } => {
    const kids = childrenOf.get(container) ?? [];
    for (const kid of kids) size.set(kid.id, kid.type === 'box' ? layoutLevel(kid.id) : sizeOf(kid));

    const top = container === undefined;
    const g = new Dagre.graphlib.Graph();
    g.setGraph({ rankdir: 'LR', nodesep: 50, ranksep: 100, marginx: top ? 24 : 0, marginy: top ? 24 : 0 });
    g.setDefaultEdgeLabel(() => ({}));
    for (const kid of kids) {
      const { w, h } = size.get(kid.id)!;
      g.setNode(kid.id, { width: w, height: h });
    }
    for (const { source, target, label } of edges) {
      const s = childIn(source, container);
      const t = childIn(target, container);
      if (s === undefined && t === undefined) continue;
      if (s === t) continue; // inside one child box: that box's own layout
      const from = s ?? IN;
      const to = t ?? OUT;
      if (!g.hasNode(from)) g.setNode(from, { width: 0, height: 0 });
      if (!g.hasNode(to)) g.setNode(to, { width: 0, height: 0 });
      // A labelled edge (one standing in for hidden datasets) gets room
      // between its ranks for the label.
      const room = typeof label === 'string'
        ? { width: label.length * EDGE_LABEL_CHAR_W + 16, height: EDGE_LABEL_H, labelpos: 'c' }
        : {};
      g.setEdge(from, to, { ...g.edge(from, to), ...room });
    }
    Dagre.layout(g);

    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    for (const kid of kids) {
      const { x, y } = g.node(kid.id);
      const { w, h } = size.get(kid.id)!;
      x0 = Math.min(x0, x - w / 2); y0 = Math.min(y0, y - h / 2);
      x1 = Math.max(x1, x + w / 2); y1 = Math.max(y1, y + h / 2);
    }
    if (top) {
      for (const kid of kids) {
        const { x, y } = g.node(kid.id);
        const { w, h } = size.get(kid.id)!;
        position.set(kid.id, { x: x - w / 2, y: y - h / 2 });
      }
      return { w: x1, h: y1 };
    }
    // Inside a box: shift so the children's bounding box starts below the
    // header, inset by the padding. The IN/OUT stand-ins take no room.
    const w = Math.max(x1 - x0 + 2 * GROUP_PAD, GROUP_MIN_W);
    const dx = (w - (x1 - x0)) / 2 - x0;
    const dy = GROUP_HEADER_H + GROUP_PAD - y0;
    for (const kid of kids) {
      const { x, y } = g.node(kid.id);
      const { w: kw, h: kh } = size.get(kid.id)!;
      position.set(kid.id, { x: x - kw / 2 + dx, y: y - kh / 2 + dy });
    }
    return { w, h: y1 - y0 + 2 * GROUP_PAD + GROUP_HEADER_H };
  };
  layoutLevel(undefined);

  const depthOf = (node: Node): number => {
    let d = 0;
    for (let p = node.parentId; p; p = byId.get(p)?.parentId) d += 1;
    return d;
  };
  const depth = new Map(nodes.map(n => [n.id, depthOf(n)]));

  const laid = nodes.map(node => {
    const pos = position.get(node.id);
    const sz = size.get(node.id);
    if (!pos || !sz) return node;
    return {
      ...node,
      position: pos,
      width: sz.w,
      height: sz.h,
      // Without `measured`, ReactFlow drops a node's handle bounds whenever the
      // node object is replaced (every status event) and only re-measures on a
      // resize, so edges to nodes that kept their DOM element vanish.
      measured: { width: sz.w, height: sz.h },
    };
  });
  // Stable sort: parents before children, otherwise the emitted order.
  return laid.sort((a, b) => depth.get(a.id)! - depth.get(b.id)!);
}
