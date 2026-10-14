/** A dataset name without its `catalog.`/`params.` root. */
export const shortName = (name: string) => name.replace(/^(catalog|params)\./, '');

/** A step's own name: the last segment of its path. */
export const localName = (path: string) => path.slice(path.lastIndexOf('/') + 1);

/**
 * The sequence a dataset belongs to: the path before its first numeric
 * segment, so `checkpoints.7` and `epochs.7.weights` give `checkpoints` and
 * `epochs`. `null` when the name has no numeric segment.
 */
export function sequenceKey(name: string): string | null {
  const parts = shortName(name).split('.');
  const i = parts.findIndex(p => /^\d+$/.test(p));
  return i > 0 ? parts.slice(0, i).join('.') : null;
}

/**
 * A dataset's name with its first numeric segment replaced by `*`
 * (`epochs.7.weights` → `epochs.*.weights`), so only members holding the same
 * field match. `null` when the name has no numeric segment.
 */
export function sequencePattern(name: string): string | null {
  const parts = shortName(name).split('.');
  const i = parts.findIndex(p => /^\d+$/.test(p));
  if (i <= 0) return null;
  parts[i] = '*';
  return parts.join('.');
}

/** The value of a dataset's first numeric segment (`checkpoints.7` → 7). */
export function sequenceIndex(name: string): number | null {
  const seg = shortName(name).split('.').find(p => /^\d+$/.test(p));
  return seg != null ? Number(seg) : null;
}

/** Natural order, so `checkpoints.10` follows `checkpoints.9`. */
export const compareNatural = (a: string, b: string) =>
  shortName(a).localeCompare(shortName(b), undefined, { numeric: true });
