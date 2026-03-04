import type { VizGraph, StatusSnapshot } from './types';

export async function fetchGraph(): Promise<VizGraph> {
  const res = await fetch('/api/graph');
  if (!res.ok) throw new Error(`GET /api/graph: ${res.status}`);
  return res.json();
}

export async function fetchDatasetHtml(id: number): Promise<string> {
  const res = await fetch(`/api/dataset/${id}/html`);
  if (res.status === 404) return '';
  if (!res.ok) throw new Error(`GET /api/dataset/${id}/html: ${res.status}`);
  return res.text();
}

export async function fetchStatus(): Promise<StatusSnapshot> {
  const res = await fetch('/api/status');
  if (!res.ok) throw new Error(`GET /api/status: ${res.status}`);
  return res.json();
}
