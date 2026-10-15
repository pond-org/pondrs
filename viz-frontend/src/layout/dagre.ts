import Dagre from '@dagrejs/dagre';
import type { Node, Edge } from '@xyflow/react';

const NODE_W = 200;
const NODE_H = 70;
const PIPE_H = 84; // collapsed groups and stubs carry a progress bar
const DS_W = 160;
const DS_H = 56;
const BUNDLE_W = 210; // sequence labels carry an index range
const BUNDLE_H = 64;

function sizeOf(node: Node): { w: number; h: number } {
  switch (node.type) {
    case 'dataset': return { w: DS_W, h: DS_H };
    case 'bundle': return { w: BUNDLE_W, h: BUNDLE_H };
    case 'pipeline':
    case 'stub': return { w: NODE_W, h: PIPE_H };
    default: return { w: NODE_W, h: NODE_H };
  }
}

export function layoutNodes(nodes: Node[], edges: Edge[]): Node[] {
  const g = new Dagre.graphlib.Graph();
  g.setGraph({ rankdir: 'LR', nodesep: 50, ranksep: 100, marginx: 24, marginy: 24 });
  g.setDefaultEdgeLabel(() => ({}));

  for (const node of nodes) {
    const { w, h } = sizeOf(node);
    g.setNode(node.id, { width: w, height: h });
  }

  for (const edge of edges) {
    g.setEdge(edge.source, edge.target);
  }

  Dagre.layout(g);

  return nodes.map(node => {
    const pos = g.node(node.id);
    if (!pos) return node;
    const { w, h } = sizeOf(node);
    return {
      ...node,
      position: { x: pos.x - w / 2, y: pos.y - h / 2 },
      width: w,
      height: h,
      // Without `measured`, ReactFlow drops a node's handle bounds whenever the
      // node object is replaced (every status event) and only re-measures on a
      // resize, so edges to nodes that kept their DOM element vanish.
      measured: { width: w, height: h },
    };
  });
}
