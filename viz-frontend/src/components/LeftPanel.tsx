import { useState } from 'react';
import type { VizGraph, VizNode, VizDataset, NodeStatus, DatasetActivity } from '../api/types';
import type { PanelSelection } from './DatasetPanel';
import { shortName, localName, compareNatural } from '../util/sequences';

interface Props {
  graph: VizGraph | null;
  selection: PanelSelection | null;
  nodeStatuses: Record<string, NodeStatus>;
  datasetActivity: Record<string, DatasetActivity>;
  onSelect: (rfId: string, selection: PanelSelection) => void;
  expandedPipelines: Set<number>;
  onTogglePipeline: (id: number) => void;
}

interface SectionProps {
  title: string;
  count: number;
  open: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}

function Section({ title, count, open, onToggle, children }: SectionProps) {
  return (
    <div>
      <button
        onClick={onToggle}
        style={{
          width: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          background: 'none',
          border: 'none',
          borderBottom: '1px solid var(--border)',
          padding: '8px 14px',
          cursor: 'pointer',
          color: 'var(--text-muted)',
          fontSize: 14,
          fontWeight: 600,
          letterSpacing: '0.05em',
          textTransform: 'uppercase',
          fontFamily: 'Inter, system-ui, sans-serif',
          textAlign: 'left',
          gap: 8,
        }}
      >
        <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontSize: 11, opacity: 0.7, transition: 'transform 0.15s', display: 'inline-block', transform: open ? 'rotate(90deg)' : 'none' }}>▶</span>
          {title}
        </span>
        <span style={{
          fontSize: 12,
          fontWeight: 500,
          color: 'var(--text-dimmer)',
          background: 'var(--bg-tag)',
          border: '1px solid var(--border-tag)',
          borderRadius: 10,
          padding: '1px 7px',
          letterSpacing: 0,
          textTransform: 'none',
        }}>
          {count}
        </span>
      </button>
      {open && (
        <div>
          {children}
        </div>
      )}
    </div>
  );
}

interface ItemProps {
  label: string;
  active: boolean;
  onClick: () => void;
  dimmed?: boolean;
  indent?: number;
}

function Item({ label, active, onClick, dimmed, indent = 0 }: ItemProps) {
  const [hovered, setHovered] = useState(false);

  return (
    <button
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        width: '100%',
        display: 'block',
        background: active
          ? 'var(--bg-node-done)'
          : hovered
            ? 'var(--bg-tag)'
            : 'none',
        border: 'none',
        borderBottom: '1px solid var(--border)',
        padding: `7px 14px 7px ${28 + indent * 16}px`,
        cursor: 'pointer',
        color: active ? 'var(--color-done)' : dimmed ? 'var(--text-dim)' : 'var(--text-sub)',
        fontSize: 15,
        fontFamily: 'Inter, system-ui, sans-serif',
        textAlign: 'left',
        whiteSpace: 'nowrap',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        transition: 'background 0.1s, color 0.1s',
      }}
      title={label}
    >
      {label}
    </button>
  );
}

interface PipelineItemProps {
  node: VizNode;
  graph: VizGraph;
  expanded: boolean;
  expandedPipelines: Set<number>;
  nodeStatuses: Record<string, NodeStatus>;
  selection: PanelSelection | null;
  onSelect: (rfId: string, selection: PanelSelection) => void;
  onTogglePipeline: (id: number) => void;
  indent: number;
}

function PipelineItem({
  node, graph, expanded, expandedPipelines, nodeStatuses,
  selection, onSelect, onTogglePipeline, indent,
}: PipelineItemProps) {
  const [hovered, setHovered] = useState(false);
  const isActive = selection?.kind === 'node' && selection.name === node.name;

  const resolveDatasets = (ids: number[]) =>
    ids.map(id => {
      const ds = graph.datasets.find(d => d.id === id);
      return { name: ds?.name ?? `dataset_${id}`, type_string: ds?.type_string ?? '' };
    });

  return (
    <>
      <button
        onClick={() => {
          if (expanded) {
            // Collapse
            onTogglePipeline(node.id);
          } else {
            // Expand + select
            onTogglePipeline(node.id);
          }
        }}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        style={{
          width: '100%',
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          background: isActive
            ? 'var(--bg-node-done)'
            : hovered
              ? 'var(--bg-tag)'
              : 'none',
          border: 'none',
          borderBottom: '1px solid var(--border)',
          padding: `7px 14px 7px ${16 + indent * 16}px`,
          cursor: 'pointer',
          color: isActive ? 'var(--color-done)' : 'var(--text-sub)',
          fontSize: 15,
          fontFamily: 'Inter, system-ui, sans-serif',
          textAlign: 'left',
          whiteSpace: 'nowrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          transition: 'background 0.1s, color 0.1s',
        }}
        title={node.name}
      >
        <span style={{
          fontSize: 11,
          opacity: 0.7,
          transition: 'transform 0.15s',
          display: 'inline-block',
          transform: expanded ? 'rotate(90deg)' : 'none',
          flexShrink: 0,
        }}>▶</span>
        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{localName(node.name)}</span>
        <span style={{
          fontSize: 11,
          color: 'var(--text-dimmer)',
          background: 'var(--bg-tag)',
          border: '1px solid var(--border-tag)',
          borderRadius: 8,
          padding: '0 5px',
          flexShrink: 0,
          marginLeft: 'auto',
        }}>
          {node.pipe_children.length}
        </span>
      </button>
      {expanded && (
        <NodeTree
          nodes={graph.nodes.filter(n => n.parent_pipe === node.id)}
          graph={graph}
          expandedPipelines={expandedPipelines}
          nodeStatuses={nodeStatuses}
          selection={selection}
          onSelect={onSelect}
          onTogglePipeline={onTogglePipeline}
          indent={indent + 1}
        />
      )}
    </>
  );
}

interface NodeTreeProps {
  nodes: VizNode[];
  graph: VizGraph;
  expandedPipelines: Set<number>;
  nodeStatuses: Record<string, NodeStatus>;
  selection: PanelSelection | null;
  onSelect: (rfId: string, selection: PanelSelection) => void;
  onTogglePipeline: (id: number) => void;
  indent: number;
}

function NodeTree({
  nodes, graph, expandedPipelines, nodeStatuses,
  selection, onSelect, onTogglePipeline, indent,
}: NodeTreeProps) {
  const resolveDatasets = (ids: number[]) =>
    ids.map(id => {
      const ds = graph.datasets.find(d => d.id === id);
      return { name: ds?.name ?? `dataset_${id}`, type_string: ds?.type_string ?? '' };
    });

  const isNodeActive = (name: string) =>
    selection?.kind === 'node' && selection.name === name;

  return (
    <>
      {nodes.map(node => {
        if (node.is_pipe) {
          return (
            <PipelineItem
              key={node.id}
              node={node}
              graph={graph}
              expanded={expandedPipelines.has(node.id)}
              expandedPipelines={expandedPipelines}
              nodeStatuses={nodeStatuses}
              selection={selection}
              onSelect={onSelect}
              onTogglePipeline={onTogglePipeline}
              indent={indent}
            />
          );
        }
        return (
          <Item
            key={node.id}
            label={localName(node.name)}
            active={isNodeActive(node.name)}
            indent={indent}
            onClick={() => onSelect(`node-${node.id}`, {
              kind: 'node',
              name: node.name,
              type_string: node.type_string,
              status: nodeStatuses[node.name] ?? null,
              inputs: resolveDatasets(node.input_dataset_ids),
              outputs: resolveDatasets(node.output_dataset_ids),
            })}
          />
        );
      })}
    </>
  );
}

type DatasetEntry =
  | { kind: 'leaf'; ds: VizDataset; label: string }
  | { kind: 'folder'; path: string; label: string; ids: number[]; children: DatasetEntry[] };

interface PathNode {
  path: string;
  ds: VizDataset | null;
  children: Map<string, PathNode>;
  /** Ids of every dataset at or below this node. */
  ids: number[];
}

/**
 * Arrange datasets as a tree over their dotted names, so every prefix shared
 * by several datasets becomes a folder: `epochs` › `0` › `metrics`, `weights`,
 * or `stores` › `north` › `inventory`. A prefix holding a single dataset is
 * not worth a level — it stays one item labelled with the remaining path —
 * and a folder whose only child is a folder merges with it (`model.layers`).
 * Top-level entries keep first-appearance order; folder contents sort
 * naturally, so `10` follows `9`.
 */
function datasetTree(datasets: VizDataset[]): DatasetEntry[] {
  const root: PathNode = { path: '', ds: null, children: new Map(), ids: [] };
  for (const ds of datasets) {
    let node = root;
    node.ids.push(ds.id);
    for (const seg of shortName(ds.name).split('.')) {
      let child = node.children.get(seg);
      if (!child) {
        child = { path: node.path ? `${node.path}.${seg}` : seg, ds: null, children: new Map(), ids: [] };
        node.children.set(seg, child);
      }
      child.ids.push(ds.id);
      node = child;
    }
    node.ds = ds;
  }

  const entriesOf = (node: PathNode, sort: boolean): DatasetEntry[] => {
    const kids = [...node.children.entries()];
    if (sort) kids.sort(([a], [b]) => compareNatural(a, b));
    return kids.flatMap(([seg, child]) => compact(child, seg));
  };

  const compact = (node: PathNode, label: string): DatasetEntry[] => {
    // A name that is both a dataset and a prefix of others shows as both.
    const own: DatasetEntry[] = node.ds && node.children.size > 0
      ? [{ kind: 'leaf', ds: node.ds, label }]
      : [];
    const below = node.ids.length - own.length;
    if (below === 1) {
      let leaf = node;
      let path = label;
      while (!leaf.ds || leaf.children.size > 0) {
        const [[seg, next]] = [...leaf.children.entries()].filter(([, c]) => c.ids.length > 0);
        path = `${path}.${seg}`;
        leaf = next;
      }
      return [...own, { kind: 'leaf', ds: leaf.ds!, label: path }];
    }
    if (!node.ds && node.children.size === 1) {
      const [[seg, only]] = [...node.children.entries()];
      return compact(only, `${label}.${seg}`);
    }
    return [...own, {
      kind: 'folder',
      path: node.path,
      label,
      ids: own.length > 0 ? node.ids.filter(id => id !== node.ds!.id) : node.ids,
      children: entriesOf(node, true),
    }];
  };

  return entriesOf(root, false);
}

interface DatasetListProps {
  datasets: VizDataset[];
  datasetActivity: Record<string, DatasetActivity>;
  selection: PanelSelection | null;
  onSelect: (rfId: string, selection: PanelSelection) => void;
  dimmed?: boolean;
}

function DatasetList({ datasets, datasetActivity, selection, onSelect, dimmed }: DatasetListProps) {
  // Folders start collapsed; toggling records an explicit choice per folder.
  const [toggled, setToggled] = useState<Record<string, boolean>>({});
  const [hoveredFolder, setHoveredFolder] = useState<string | null>(null);

  const isActive = (id: number) => selection?.kind === 'dataset' && selection.id === id;

  const item = (ds: VizDataset, label: string, indent: number) => (
    <Item
      key={ds.id}
      label={label}
      active={isActive(ds.id)}
      dimmed={dimmed}
      indent={indent}
      onClick={() => onSelect(`ds-${ds.id}`, {
        kind: 'dataset',
        id: ds.id,
        name: ds.name,
        type_string: ds.type_string,
        is_param: ds.is_param,
        activity: datasetActivity[ds.name] ?? null,
      })}
    />
  );

  const render = (entry: DatasetEntry, indent: number): React.ReactNode => {
    if (entry.kind === 'leaf') return item(entry.ds, entry.label, indent);

    const { path, label, ids, children } = entry;
    // Selecting a member (e.g. by clicking it in the graph) reveals it,
    // unless the user has explicitly collapsed the folder.
    const containsActive = ids.some(isActive);
    const open = toggled[path] ?? containsActive;
    return (
      <div key={`folder-${path}`}>
        <button
          onClick={() => setToggled(t => ({ ...t, [path]: !open }))}
          onMouseEnter={() => setHoveredFolder(path)}
          onMouseLeave={() => setHoveredFolder(null)}
          style={{
            width: '100%',
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            background: containsActive && !open
              ? 'var(--bg-node-done)'
              : hoveredFolder === path ? 'var(--bg-tag)' : 'none',
            border: 'none',
            borderBottom: '1px solid var(--border)',
            padding: `7px 14px 7px ${16 + indent * 16}px`,
            cursor: 'pointer',
            color: containsActive && !open ? 'var(--color-done)' : dimmed ? 'var(--text-dim)' : 'var(--text-sub)',
            fontSize: 15,
            fontFamily: 'Inter, system-ui, sans-serif',
            textAlign: 'left',
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            transition: 'background 0.1s, color 0.1s',
          }}
          title={`${path} — ${ids.length} datasets`}
        >
          <span style={{
            fontSize: 11,
            opacity: 0.7,
            transition: 'transform 0.15s',
            display: 'inline-block',
            transform: open ? 'rotate(90deg)' : 'none',
            flexShrink: 0,
          }}>▶</span>
          <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{label}</span>
          <span style={{
            fontSize: 11,
            color: 'var(--text-dimmer)',
            background: 'var(--bg-tag)',
            border: '1px solid var(--border-tag)',
            borderRadius: 8,
            padding: '0 5px',
            flexShrink: 0,
            marginLeft: 'auto',
          }}>
            {ids.length}
          </span>
        </button>
        {open && children.map(c => render(c, indent + 1))}
      </div>
    );
  };

  if (datasets.length === 0) {
    return <div style={{ padding: '8px 14px', fontSize: 14, color: 'var(--text-dim)' }}>—</div>;
  }

  return <>{datasetTree(datasets).map(e => render(e, 0))}</>;
}

export function LeftPanel({
  graph, selection, nodeStatuses, datasetActivity,
  onSelect, expandedPipelines, onTogglePipeline,
}: Props) {
  const [nodesOpen, setNodesOpen] = useState(true);
  const [datasetsOpen, setDatasetsOpen] = useState(true);
  const [paramsOpen, setParamsOpen] = useState(true);

  // Top-level nodes: those with no parent pipeline.
  const topLevelNodes = graph?.nodes.filter(n => n.parent_pipe == null) ?? [];
  const datasets = graph?.datasets.filter(d => !d.is_param) ?? [];
  const params = graph?.datasets.filter(d => d.is_param) ?? [];

  // Count all non-pipe nodes for the section header.
  const totalStepCount = graph?.nodes.filter(n => !n.is_pipe).length ?? 0;

  return (
    <div style={{
      width: '100%',
      height: '100%',
      background: 'var(--bg-panel)',
      borderRight: '1px solid var(--border)',
      display: 'flex',
      flexDirection: 'column',
      overflowY: 'auto',
    }}>
      <Section
        title="Nodes"
        count={totalStepCount}
        open={nodesOpen}
        onToggle={() => setNodesOpen(o => !o)}
      >
        {topLevelNodes.length === 0 ? (
          <div style={{ padding: '8px 14px', fontSize: 14, color: 'var(--text-dim)' }}>—</div>
        ) : (
          graph && (
            <NodeTree
              nodes={topLevelNodes}
              graph={graph}
              expandedPipelines={expandedPipelines}
              nodeStatuses={nodeStatuses}
              selection={selection}
              onSelect={onSelect}
              onTogglePipeline={onTogglePipeline}
              indent={0}
            />
          )
        )}
      </Section>

      <Section
        title="Datasets"
        count={datasets.length}
        open={datasetsOpen}
        onToggle={() => setDatasetsOpen(o => !o)}
      >
        <DatasetList
          datasets={datasets}
          datasetActivity={datasetActivity}
          selection={selection}
          onSelect={onSelect}
        />
      </Section>

      <Section
        title="Parameters"
        count={params.length}
        open={paramsOpen}
        onToggle={() => setParamsOpen(o => !o)}
      >
        <DatasetList
          datasets={params}
          datasetActivity={datasetActivity}
          selection={selection}
          onSelect={onSelect}
          dimmed
        />
      </Section>
    </div>
  );
}
