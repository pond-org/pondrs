import Dagre from '@dagrejs/dagre';
import type { Node, Edge } from '@xyflow/react';

const NODE_W = 200;
const NODE_H = 70;
const DS_W = 160;
const DS_H = 56;

export function layoutNodes(nodes: Node[], edges: Edge[]): Node[] {
  const g = new Dagre.graphlib.Graph({ compound: true });
  g.setGraph({ rankdir: 'LR', nodesep: 50, ranksep: 100, marginx: 24, marginy: 24 });
  g.setDefaultEdgeLabel(() => ({}));

  for (const node of nodes) {
    const w = node.type === 'dataset' ? DS_W : NODE_W;
    const h = node.type === 'dataset' ? DS_H : NODE_H;
    g.setNode(node.id, { width: w, height: h });
    if (node.parentId) {
      g.setParent(node.id, node.parentId);
    }
  }

  for (const edge of edges) {
    g.setEdge(edge.source, edge.target);
  }

  Dagre.layout(g);

  return nodes.map(node => {
    const pos = g.node(node.id);
    if (!pos) return node;
    const w = node.type === 'dataset' ? DS_W : NODE_W;
    const h = node.type === 'dataset' ? DS_H : NODE_H;
    return {
      ...node,
      position: { x: pos.x - w / 2, y: pos.y - h / 2 },
      width: w,
      height: h,
    };
  });
}
