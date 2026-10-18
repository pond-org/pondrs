import { useRef, useState, type CSSProperties } from 'react';
import type { PagerInfo } from '../hooks/useGraph';
import { PAGE_SIZES, type PageState } from '../util/paging';
import { localName } from '../util/sequences';

interface Props {
  pagers: PagerInfo[];
  onPageChange: (groupId: number, patch: Partial<PageState>) => void;
}

const button: CSSProperties = {
  background: 'var(--bg-tag)',
  border: '1px solid var(--border-tag)',
  borderRadius: 6,
  color: 'var(--text-muted)',
  cursor: 'pointer',
  fontSize: 14,
  lineHeight: 1,
  padding: '4px 8px',
};

const active: CSSProperties = {
  ...button,
  background: 'var(--bg-node-run)',
  borderColor: 'var(--color-running)',
  color: 'var(--color-running)',
};

/**
 * Jump to an exact index. A plain text field (no spinner — ◀ ▶ step) that
 * commits on Enter or blur, so typing `120` doesn't visit 1 and 12 first.
 */
function IndexInput({ value, onCommit }: { value: number; onCommit: (v: number) => void }) {
  // `null` while not editing, so the field tracks the window (e.g. when following).
  const [draft, setDraft] = useState<string | null>(null);
  // Escape blurs too, and blur runs before the draft reset lands, so flag it.
  const cancelled = useRef(false);
  const commit = () => {
    if (!cancelled.current && draft != null && draft !== '') onCommit(Number(draft));
    cancelled.current = false;
    setDraft(null);
  };
  return (
    <input
      type="text"
      inputMode="numeric"
      value={draft ?? String(value)}
      onFocus={e => { setDraft(String(value)); e.target.select(); }}
      onChange={e => setDraft(e.target.value.replace(/[^0-9]/g, ''))}
      onBlur={commit}
      onKeyDown={e => {
        if (e.key === 'Enter') e.currentTarget.blur();
        else if (e.key === 'Escape') { cancelled.current = true; e.currentTarget.blur(); }
      }}
      title="Jump to index (Enter)"
      style={{
        width: 56,
        background: 'var(--bg-panel)',
        border: '1px solid var(--border-tag)',
        borderRadius: 6,
        color: 'var(--text)',
        fontSize: 14,
        padding: '3px 6px',
        textAlign: 'center',
      }}
    />
  );
}

function Pager({ p, onPageChange }: { p: PagerInfo; onPageChange: Props['onPageChange'] }) {
  const total = p.children.length;
  const last = total - p.size;
  // Manual navigation stops following; the follow button resumes it.
  const go = (start: number) =>
    onPageChange(p.groupId, { start: Math.max(0, Math.min(last, start)), follow: false });
  const shown = p.size === 1
    ? p.children[p.start]
    : `${p.children[p.start]} – ${p.children[p.start + p.size - 1]}`;
  const targetOutside = p.target != null && (p.target < p.start || p.target >= p.start + p.size);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
        <span title={p.name} style={{ fontWeight: 600, color: 'var(--text)' }}>{localName(p.name)}</span>
        <span style={{ color: 'var(--text-muted)', fontSize: 14 }}>
          {shown} <span style={{ color: 'var(--text-dim)' }}>of {total}</span>
        </span>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <button style={button} onClick={() => go(p.start - p.size)} disabled={p.start === 0} title="Previous page">◀</button>
        <IndexInput value={p.start} onCommit={go} />
        <button style={button} onClick={() => go(p.start + p.size)} disabled={p.start >= last} title="Next page">▶</button>
        <input
          type="range"
          min={0}
          max={Math.max(0, last)}
          value={p.start}
          onChange={e => go(Number(e.target.value))}
          style={{ flex: 1, minWidth: 100, accentColor: 'var(--color-running)' }}
        />
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <span style={{ color: 'var(--text-dim)', fontSize: 13 }}>show</span>
        {PAGE_SIZES.map(size => (
          <button
            key={size}
            style={p.size === size ? active : button}
            onClick={() => onPageChange(p.groupId, { size, start: p.start })}
          >
            {size}
          </button>
        ))}
        <div style={{ flex: 1 }} />
        <button
          style={p.follow ? active : button}
          onClick={() => onPageChange(p.groupId, { follow: !p.follow, start: p.start })}
          title={p.follow ? 'Stop following the running step' : 'Follow the running step'}
        >
          ⟳ follow{targetOutside && !p.follow ? ` (${p.children[p.target!]})` : ''}
        </button>
      </div>
    </div>
  );
}

/** Controls for every expanded group that is too large to show whole. */
export function GroupPager({ pagers, onPageChange }: Props) {
  if (pagers.length === 0) return null;
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 12,
        background: 'var(--bg-panel)',
        border: '1px solid var(--border)',
        borderRadius: 10,
        padding: '10px 12px',
        width: 300,
        maxHeight: '60vh',
        overflowY: 'auto',
        fontFamily: 'Inter, system-ui, sans-serif',
        fontSize: 15,
        boxShadow: '0 4px 16px rgba(0,0,0,0.25)',
      }}
    >
      {pagers.map(p => <Pager key={p.groupId} p={p} onPageChange={onPageChange} />)}
    </div>
  );
}
