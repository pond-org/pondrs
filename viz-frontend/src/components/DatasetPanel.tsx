import { useEffect, useState } from 'react';
import { fetchDatasetHtml } from '../api/client';
import type { NodeStatus, DatasetActivity } from '../api/types';

export type PanelSelection =
  | { kind: 'dataset'; id: number; name: string; is_param: boolean; activity: DatasetActivity | null }
  | { kind: 'node'; name: string; status: NodeStatus | null };

interface Props {
  selection: PanelSelection | null;
  onClose: () => void;
  isDark: boolean;
}

const STATUS_COLOR: Record<string, string> = {
  pending: 'var(--text-dim)',
  running: 'var(--color-running)',
  completed: 'var(--color-done)',
  error: 'var(--color-error)',
};

function buildIframeCss(isDark: boolean): string {
  if (isDark) {
    return `
      html, body {
        background: #141414;
        color: #e5e5e5;
        font-family: 'Inter', system-ui, sans-serif;
        font-size: 15px;
        margin: 0;
        padding: 10px;
      }
      table { border-collapse: collapse; width: 100%; font-size: 15px; }
      th {
        background: #1e1e1e !important;
        color: #e5e5e5 !important;
        border: 1px solid #383838 !important;
        padding: 6px 12px;
        text-align: left;
        font-family: 'Inter', monospace;
        font-weight: 600;
      }
      td {
        border: 1px solid #333 !important;
        color: #e5e5e5 !important;
        background: #141414 !important;
        padding: 6px 12px;
        font-family: 'Fira Mono', 'Consolas', monospace;
      }
      tr:hover td { background: #1a1a1a !important; }
      pre {
        background: #1a1a1a !important;
        color: #e5e5e5 !important;
        border: 1px solid #383838 !important;
        padding: 14px;
        border-radius: 6px;
        font-size: 14px;
        line-height: 1.6;
        overflow: auto;
      }
      p { color: #888 !important; font-size: 13px; margin-top: 8px; }
    `;
  } else {
    return `
      html, body {
        background: #ffffff;
        color: #111111;
        font-family: 'Inter', system-ui, sans-serif;
        font-size: 15px;
        margin: 0;
        padding: 10px;
      }
      table { border-collapse: collapse; width: 100%; font-size: 15px; }
      th {
        background: #f5f5f5 !important;
        color: #111111 !important;
        border: 1px solid #dddddd !important;
        padding: 6px 12px;
        text-align: left;
        font-family: 'Inter', monospace;
        font-weight: 600;
      }
      td {
        border: 1px solid #e0e0e0 !important;
        color: #111111 !important;
        background: #ffffff !important;
        padding: 6px 12px;
        font-family: 'Fira Mono', 'Consolas', monospace;
      }
      tr:hover td { background: #f9f9f9 !important; }
      pre {
        background: #f5f5f5 !important;
        color: #111111 !important;
        border: 1px solid #dddddd !important;
        padding: 14px;
        border-radius: 6px;
        font-size: 14px;
        line-height: 1.6;
        overflow: auto;
      }
      p { color: #666 !important; font-size: 13px; margin-top: 8px; }
    `;
  }
}

function buildPlotlyJs(isDark: boolean): string {
  if (!isDark) return '';
  return `
    <script>
    (function() {
      var bg = '#141414', plotBg = '#1a1a1a', textColor = '#e5e5e5', gridColor = '#333333';
      function applyTheme() {
        var plots = document.getElementsByClassName('plotly-graph-div');
        if (!window.Plotly || plots.length === 0) {
          setTimeout(applyTheme, 80);
          return;
        }
        for (var i = 0; i < plots.length; i++) {
          try {
            Plotly.relayout(plots[i], {
              paper_bgcolor: bg,
              plot_bgcolor: plotBg,
              font: { color: textColor },
              xaxis: { gridcolor: gridColor, linecolor: '#555', tickfont: { color: '#aaa' }, zerolinecolor: '#555' },
              yaxis: { gridcolor: gridColor, linecolor: '#555', tickfont: { color: '#aaa' }, zerolinecolor: '#555' },
              legend: { bgcolor: '#1e1e1e', bordercolor: '#333' }
            });
          } catch(e) {}
        }
      }
      if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', applyTheme);
      } else {
        applyTheme();
      }
    })();
    </script>
  `;
}

function injectTheme(html: string, isDark: boolean): string {
  const css = buildIframeCss(isDark);
  const styleTag = `<style>${css}</style>`;
  const plotlyJs = buildPlotlyJs(isDark);

  // Full HTML document (e.g. Plotly output)
  if (/<html[\s>]/i.test(html)) {
    let result = html;
    if (/<head[\s>]/i.test(result)) {
      result = result.replace(/<head([^>]*)>/i, `<head$1>${styleTag}`);
    } else {
      result = result.replace(/<html([^>]*)>/i, `<html$1><head>${styleTag}</head>`);
    }
    // Inject Plotly dark theme script before </body>
    if (plotlyJs) {
      result = result.includes('</body>')
        ? result.replace('</body>', `${plotlyJs}</body>`)
        : result + plotlyJs;
    }
    return result;
  }

  // Partial HTML snippet (table, pre, etc.) — wrap in a minimal document
  return `<!DOCTYPE html><html><head><meta charset="UTF-8">${styleTag}</head><body>${html}</body></html>`;
}

export function DatasetPanel({ selection, onClose, isDark }: Props) {
  const [html, setHtml] = useState('');
  const [loading, setLoading] = useState(false);

  const datasetId = selection?.kind === 'dataset' ? selection.id : null;

  useEffect(() => {
    if (datasetId == null) { setHtml(''); return; }
    setLoading(true);
    fetchDatasetHtml(datasetId)
      .then(h => { setHtml(h); setLoading(false); })
      .catch(() => { setHtml(''); setLoading(false); });
  }, [datasetId]);

  const themedHtml = html ? injectTheme(html, isDark) : '';

  const open = selection != null;

  return (
    <div style={{
      position: 'absolute',
      top: 0,
      right: 0,
      width: 520,
      height: '100%',
      background: 'var(--bg-panel)',
      borderLeft: '1px solid var(--border-sub)',
      display: 'flex',
      flexDirection: 'column',
      zIndex: 10,
      transform: open ? 'translateX(0)' : 'translateX(100%)',
      transition: 'transform 0.2s ease',
      pointerEvents: open ? 'auto' : 'none',
    }}>
      {/* Header */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '12px 18px',
        borderBottom: '1px solid var(--border-sub)',
        gap: 10,
        flexShrink: 0,
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, overflow: 'hidden' }}>
          {selection && (
            <span style={{
              fontSize: 15,
              color: 'var(--text-muted)',
              background: 'var(--bg-tag)',
              padding: '3px 9px',
              borderRadius: 5,
              flexShrink: 0,
              border: '1px solid var(--border-tag)',
            }}>
              {selection.kind === 'node' ? 'node' : selection.is_param ? 'param' : 'dataset'}
            </span>
          )}
          <span style={{
            color: 'var(--text-sub)',
            fontSize: 20,
            fontWeight: 600,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}>
            {selection?.kind === 'dataset'
              ? selection.name.replace(/^(catalog|params)\./, '')
              : selection?.name ?? ''}
          </span>
        </div>
        <button
          onClick={onClose}
          style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', fontSize: 27, lineHeight: 1, flexShrink: 0, padding: '0 4px' }}
        >
          ×
        </button>
      </div>

      {/* Node body */}
      {selection?.kind === 'node' && (
        <div style={{ padding: 20, overflowY: 'auto' }}>
          <NodeInfo status={selection.status} />
        </div>
      )}

      {/* Dataset body */}
      {selection?.kind === 'dataset' && (
        <>
          {selection.activity && (
            <div style={{ padding: '10px 18px', borderBottom: '1px solid var(--border-sub)', display: 'flex', gap: 20, flexShrink: 0 }}>
              {selection.activity.load_ms != null && (
                <span style={{ fontSize: 17, color: 'var(--text-muted)' }}>↓ load: {selection.activity.load_ms.toFixed(1)}ms</span>
              )}
              {selection.activity.save_ms != null && (
                <span style={{ fontSize: 17, color: 'var(--text-muted)' }}>↑ save: {selection.activity.save_ms.toFixed(1)}ms</span>
              )}
            </div>
          )}
          <div style={{ flex: 1, overflow: 'hidden' }}>
            {loading && (
              <div style={{ color: 'var(--text-dim)', fontSize: 20, padding: 20 }}>Loading…</div>
            )}
            {!loading && !html && (
              <div style={{ color: 'var(--text-dim)', fontSize: 20, padding: 20 }}>No preview available.</div>
            )}
            {!loading && html && (
              <iframe
                srcDoc={themedHtml}
                sandbox="allow-scripts allow-same-origin"
                style={{ width: '100%', height: '100%', border: 'none', background: isDark ? '#141414' : '#ffffff' }}
                title="dataset preview"
              />
            )}
          </div>
        </>
      )}
    </div>
  );
}

function NodeInfo({ status }: { status: NodeStatus | null }) {
  if (!status || status.status === 'pending') {
    return <div style={{ color: 'var(--text-dim)', fontSize: 20 }}>No execution data yet.</div>;
  }

  const color = STATUS_COLOR[status.status] ?? 'var(--text-dim)';
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <span style={{
          fontSize: 17,
          color,
          background: 'var(--bg-tag)',
          border: `1px solid var(--border-tag)`,
          padding: '3px 12px',
          borderRadius: 5,
          fontWeight: 600,
        }}>
          {status.status}
        </span>
        {status.duration_ms != null && (
          <span style={{ fontSize: 17, color: 'var(--text-muted)' }}>{status.duration_ms.toFixed(1)}ms</span>
        )}
      </div>
      {status.error && (
        <div style={{
          fontSize: 17,
          color: 'var(--color-error)',
          background: 'var(--bg-err-panel)',
          border: '1px solid var(--border-err)',
          borderRadius: 5,
          padding: '10px 14px',
          wordBreak: 'break-word',
        }}>
          {status.error}
        </div>
      )}
    </div>
  );
}
