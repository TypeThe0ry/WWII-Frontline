/**
 * Manifest <-> `.fpak` index consistency (M1-005): every `pak/entry/offset/bytes/sha256` in the manifest must equal what the
 * pack's own index and payload say; `placeholder:false` needs a ledger row with status OK.
 */
import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { FileSource, FpakReader } from './fpak/index.ts';
import { loadLedger, type Ledger } from './ledger.ts';
import { assetRefs, type Manifest } from './manifest.ts';

export async function checkConsistency(m: Manifest, packs: string): Promise<string[]> {
  const problems: string[] = [];
  const readers = new Map<string, { r: FpakReader; src: FileSource } | null>();
  try {
    for (const [id, a] of Object.entries(m.assets)) {
      for (const ref of assetRefs(a)) {
        let h = readers.get(ref.pak);
        if (h === undefined) {
          const file = join(packs, ref.pak);
          if (existsSync(file)) {
            const src = new FileSource(file);
            h = { r: await FpakReader.open(src), src };
          } else h = null;
          readers.set(ref.pak, h);
        }
        if (!h) {
          problems.push(`${id} (${ref.entry}): pack ${ref.pak} not found`);
          continue;
        }
        if (!h.r.has(ref.entry)) {
          problems.push(`${id} (${ref.entry}): entry missing in ${ref.pak}`);
          continue;
        }
        const e = h.r.entry(ref.entry);
        if (Number(e.offset) !== ref.offset) problems.push(`${id} (${ref.entry}): offset ${ref.offset} vs index ${e.offset}`);
        if (e.bytes !== ref.bytes) problems.push(`${id} (${ref.entry}): bytes ${ref.bytes} vs index ${e.bytes}`);
        else {
          const sha = createHash('sha256').update(await h.r.read(ref.entry)).digest('hex');
          if (sha !== ref.sha256) problems.push(`${id} (${ref.entry}): sha256 differs from the payload`);
        }
      }
    }
  } finally {
    for (const h of readers.values()) h?.src.close();
  }
  return problems;
}

/** Internal assets are never distributable; other `placeholder:false` entries require status OK. */
export function checkPlaceholders(m: Manifest, ledger: string | Ledger): string[] {
  const l = typeof ledger === 'string' ? loadLedger(ledger) : ledger;
  const problems: string[] = [];
  for (const [id, a] of Object.entries(m.assets)) {
    const assetId = a.license_ref?.replace(/^LICENSES\.md#/, '');
    const row = assetId ? l.byAssetId.get(assetId) : undefined;
    if (row?.status === 'INTERNAL') problems.push(`${id}: internal-only ledger asset ${row.asset_id} is not distributable`);
    else if (a.placeholder) continue;
    else if (!row) problems.push(`${id}: placeholder:false without a ledger row`);
    else if (row.status !== 'OK') problems.push(`${id}: placeholder:false but ledger ${row.asset_id} is ${row.status}`);
  }
  return problems;
}
