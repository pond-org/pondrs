import { useState } from 'react';
import type { VizGraph, NodeStatus, DatasetActivity } from '../api/types';
import type { PanelSelection } from './DatasetPanel';

interface Props {
  graph: VizGraph | null;
  selection: PanelSelection | null;
  nodeStatuses: Record<string, NodeStatus>;
  datasetActivity: Record<string, DatasetActivity>;
  onSelect: (rfId: string, selection: PanelSelection) => void;
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
}

function Item({ label, active, onClick, dimmed }: ItemProps) {
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
        padding: '7px 14px 7px 28px',
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

export function LeftPanel({ graph, selection, nodeStatuses, datasetActivity, onSelect }: Props) {
  const [nodesOpen, setNodesOpen] = useState(true);
  const [datasetsOpen, setDatasetsOpen] = useState(true);
  const [paramsOpen, setParamsOpen] = useState(true);

  const stepNodes = graph?.nodes.filter(n => !n.is_pipe) ?? [];
  const datasets = graph?.datasets.filter(d => !d.is_param) ?? [];
  const params = graph?.datasets.filter(d => d.is_param) ?? [];

  const isNodeActive = (name: string) =>
    selection?.kind === 'node' && selection.name === name;
  const isDatasetActive = (id: number) =>
    selection?.kind === 'dataset' && selection.id === id;

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
        count={stepNodes.length}
        open={nodesOpen}
        onToggle={() => setNodesOpen(o => !o)}
      >
        {stepNodes.length === 0 ? (
          <div style={{ padding: '8px 14px', fontSize: 14, color: 'var(--text-dim)' }}>—</div>
        ) : (
          stepNodes.map(node => (
            <Item
              key={node.id}
              label={node.name}
              active={isNodeActive(node.name)}
              onClick={() => onSelect(`node-${node.id}`, {
                kind: 'node',
                name: node.name,
                type_string: node.type_string,
                status: nodeStatuses[node.name] ?? null,
              })}
            />
          ))
        )}
      </Section>

      <Section
        title="Datasets"
        count={datasets.length}
        open={datasetsOpen}
        onToggle={() => setDatasetsOpen(o => !o)}
      >
        {datasets.length === 0 ? (
          <div style={{ padding: '8px 14px', fontSize: 14, color: 'var(--text-dim)' }}>—</div>
        ) : (
          datasets.map(ds => (
            <Item
              key={ds.id}
              label={ds.name.replace(/^(catalog|params)\./, '')}
              active={isDatasetActive(ds.id)}
              onClick={() => onSelect(`ds-${ds.id}`, {
                kind: 'dataset',
                id: ds.id,
                name: ds.name,
                type_string: ds.type_string,
                is_param: ds.is_param,
                activity: datasetActivity[ds.name] ?? null,
              })}
            />
          ))
        )}
      </Section>

      <Section
        title="Parameters"
        count={params.length}
        open={paramsOpen}
        onToggle={() => setParamsOpen(o => !o)}
      >
        {params.length === 0 ? (
          <div style={{ padding: '8px 14px', fontSize: 14, color: 'var(--text-dim)' }}>—</div>
        ) : (
          params.map(ds => (
            <Item
              key={ds.id}
              label={ds.name.replace(/^(catalog|params)\./, '')}
              active={isDatasetActive(ds.id)}
              dimmed
              onClick={() => onSelect(`ds-${ds.id}`, {
                kind: 'dataset',
                id: ds.id,
                name: ds.name,
                type_string: ds.type_string,
                is_param: ds.is_param,
                activity: datasetActivity[ds.name] ?? null,
              })}
            />
          ))
        )}
      </Section>
    </div>
  );
}
