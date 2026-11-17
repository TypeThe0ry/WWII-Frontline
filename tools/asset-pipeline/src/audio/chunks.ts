/** Metadata self-check (ASSET_SHOPPING_LIST 3.4): output must carry no iXML / bext / SMED / LIST-INFO data. */
export const FORBIDDEN = ['iXML', 'bext', 'SMED'] as const;

export function forbiddenChunks(bytes: Uint8Array): string[] {
  const s = Buffer.from(bytes.buffer, bytes.byteOffset, bytes.byteLength).toString('latin1');
  const hits: string[] = FORBIDDEN.filter((t) => s.includes(t));
  if (/LIST[\s\S]{4}INFO/.test(s)) hits.push('LIST/INFO');
  return hits;
}

export function assertNoForbiddenChunks(bytes: Uint8Array, what: string): void {
  const h = forbiddenChunks(bytes);
  if (h.length) throw new Error(`${what}: source metadata found in output (${h.join(', ')})`);
}
