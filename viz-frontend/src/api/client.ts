import type { VizGraph, StatusSnapshot } from './types';

interface StaticData {
  graph: VizGraph;
  datasetHtml: Record<number, string>;
  datasetYaml: Record<number, string>;
}

function getStaticData(): StaticData | null {
  return (window as any).__STATIC_DATA__ ?? null;
}

export function isStaticMode(): boolean {
  return getStaticData() !== null;
}

export async function fetchGraph(): Promise<VizGraph> {
  const sd = getStaticData();
  if (sd) return sd.graph;
  const res = await fetch('/api/graph');
  if (!res.ok) throw new Error(`GET /api/graph: ${res.status}`);
  return res.json();
}

export async function fetchDatasetHtml(id: number): Promise<string> {
  const sd = getStaticData();
  if (sd) return sd.datasetHtml[id] ?? '';
  const res = await fetch(`/api/dataset/${id}/html`);
  if (res.status === 404) return '';
  if (!res.ok) throw new Error(`GET /api/dataset/${id}/html: ${res.status}`);
  return res.text();
}

export async function fetchDatasetYaml(id: number): Promise<string> {
  const sd = getStaticData();
  if (sd) return sd.datasetYaml[id] ?? '';
  const res = await fetch(`/api/dataset/${id}/yaml`);
  if (res.status === 404) return '';
  if (!res.ok) throw new Error(`GET /api/dataset/${id}/yaml: ${res.status}`);
  return res.text();
}

export async function fetchStatus(): Promise<StatusSnapshot> {
  const sd = getStaticData();
  if (sd) return { nodes: {}, datasets: {} };
  const res = await fetch('/api/status');
  if (!res.ok) throw new Error(`GET /api/status: ${res.status}`);
  return res.json();
}
