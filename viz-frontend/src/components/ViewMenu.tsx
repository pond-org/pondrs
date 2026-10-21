import { useEffect, useRef, useState } from 'react';
import type { ViewOptions } from '../util/view';

type Switch = 'boxes' | 'datasets' | 'params';

const SWITCHES: { key: Switch; label: string; hint: string }[] = [
  { key: 'boxes', label: 'boxes', hint: 'Draw expanded groups as boxes' },
  { key: 'datasets', label: 'datasets', hint: 'Show datasets passed between steps; off joins producer to consumer' },
  { key: 'params', label: 'params', hint: 'Show params' },
];

const buttonStyle = {
  background: 'var(--bg-tag)',
  border: '1px solid var(--border-tag)',
  borderRadius: 8,
  color: 'var(--text-muted)',
  cursor: 'pointer',
  lineHeight: 1,
} as const;

interface Props {
  view: ViewOptions;
  onChange: <K extends keyof ViewOptions>(key: K, value: ViewOptions[K]) => void;
}

/** Header controls: a menu of canvas switches, plus a theme shortcut. */
export function ViewMenu({ view, onChange }: Props) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const dark = view.theme === 'dark';

  // Close on a click outside the menu or on Escape.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as globalThis.Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [open]);

  return (
    <div style={{ display: 'flex', gap: 8 }}>
      <div ref={ref} style={{ position: 'relative' }}>
        <button
          onClick={() => setOpen(o => !o)}
          title="View options"
          aria-expanded={open}
          style={{ ...buttonStyle, fontSize: 15, padding: '6px 10px' }}
        >
          view ▾
        </button>
        {open && (
          <div
            role="menu"
            style={{
              position: 'absolute',
              top: 'calc(100% + 6px)',
              right: 0,
              minWidth: 180,
              background: 'var(--bg-panel)',
              border: '1px solid var(--border-tag)',
              borderRadius: 8,
              boxShadow: '0 6px 20px rgba(0, 0, 0, 0.25)',
              padding: 6,
              zIndex: 10,
            }}
          >
            {SWITCHES.map(({ key, label, hint }) => (
              <label
                key={key}
                title={hint}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  padding: '7px 8px',
                  borderRadius: 6,
                  cursor: 'pointer',
                  color: view[key] ? 'var(--text-sub)' : 'var(--text-muted)',
                  fontSize: 15,
                }}
              >
                <input
                  type="checkbox"
                  checked={view[key]}
                  onChange={e => onChange(key, e.target.checked)}
                  style={{ accentColor: 'var(--pipeline-border)', margin: 0 }}
                />
                {label}
              </label>
            ))}
          </div>
        )}
      </div>
      <button
        onClick={() => onChange('theme', dark ? 'light' : 'dark')}
        title={dark ? 'Switch to light mode' : 'Switch to dark mode'}
        style={{ ...buttonStyle, fontSize: 18, padding: '4px 10px' }}
      >
        {dark ? '☀' : '☾'}
      </button>
    </div>
  );
}
