/** Box chain of a canvas node, outermost first: ['group-7', 'group-9']. */
export function chain(box: string | undefined, parentOf: Map<string, string | undefined>): string[] {
  const out: string[] = [];
  for (let b = box; b !== undefined; b = parentOf.get(b)) out.push(b);
  return out.reverse();
}

/** Deepest box shared by all chains; undefined = top level. */
export function commonBox(chains: string[][]): string | undefined {
  if (chains.length === 0) return undefined;
  const first = chains[0];
  let n = 0;
  while (n < first.length && chains.every(c => c[n] === first[n])) n += 1;
  return n === 0 ? undefined : first[n - 1];
}
