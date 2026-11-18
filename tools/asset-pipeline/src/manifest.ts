/**
 * `manifest` (ARCHITECTURE 11.1 step 7 / 11.3, M1-005): `.fpak` files + `work/*\/stats.json` + ledger -> `manifest.json`.
 *
 * The manifest is derived from the published packs themselves (every `.fpak` under the pack directory is opened and its
 * index read), so `pak/entry/offset/bytes/sha256` are right by construction; `consistency.ts` re-checks them later.
 * Output is deterministic: keys sorted, entries sorted, only `build` carries a timestamp.
 */
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, sep } from 'node:path';
import { FileSource, FpakReader, INDEX_FIXED_BYTES, type KindName } from './fpak/index.ts';
import { assertStatusOk } from './gate.ts';
import { loadLedger, type Ledger } from './ledger.ts';
import { readStats, type Stats } from './stats.ts';

export const MANIFEST_SCHEMA = 1;
export const TIERS = ['low', 'medium', 'high', 'ultra'] as const;
export type Tier = (typeof TIERS)[number];

export interface ManifestEntryRef {
  pak: string;
  entry: string;
  offset: number;
  bytes: number;
  sha256: string;
  /** Index bytes this entry costs inside its pack (fixed part + id). */
  index_bytes: number;
}

export interface ManifestLod extends ManifestEntryRef {
  lod: number;
  /** '' | '_fp' | '_tp'. */
  suffix: string;
  tris?: number;
}

export interface ManifestAsset {
  kind: 'model' | 'texture' | 'audio' | 'data';
  tier: Tier[];
  placeholder: boolean;
  /** Ledger asset id (`license_ref` = `LICENSES.md#{asset_id}`), absent for placeholders without a ledger row. */
  license_ref?: string;
  req_id?: string;
  lods?: ManifestLod[];
  collision?: ManifestEntryRef;
  textures?: (ManifestEntryRef & { name: string })[];
  audio?: (ManifestEntryRef & { codec: 'opus' | 'aac' })[];
  meta?: ManifestEntryRef;
}

export interface Manifest {
  schema: number;
  build: string;
  game_version: string;
  data_hash: string;
  assets: Record<string, ManifestAsset>;
  bundles: Record<string, string[]>;
  budgets: Record<string, number>;
}

export interface Overrides {
  /** Per logical id or glob (`veh_*`): tier list, placeholder flag, requirement id. */
  [pattern: string]: { tier?: Tier[]; placeholder?: boolean; req_id?: string };
}

export interface BuildManifestOptions {
  /** Directory holding `a/xx/yyyy.fpak`. */
  packs: string;
  work: string;
  ledger: string | Ledger;
  bundles: Record<string, string[]>;
  budgets: Record<string, number>;
  gameVersion?: string;
  dataHash?: string;
  build?: string;
  overrides?: Overrides;
}

/** `*` matches within a path segment-free id (any chars except `/`), `**` anything; ids use `/` only for terrain chunks. */
export function globToRegExp(glob: string): RegExp {
  const esc = glob
    .split('**')
    .map((part) => part.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '[^/]*'))
    .join('.*');
  return new RegExp(`^${esc}$`);
}

export function matchesAny(id: string, globs: readonly string[]): boolean {
  return globs.some((g) => globToRegExp(g).test(id));
}

function listPaks(dir: string): string[] {
  const out: string[] = [];
  const walk = (d: string) => {
    for (const e of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, e.name);
      if (e.isDirectory()) walk(p);
      else if (e.name.endsWith('.fpak')) out.push(p);
    }
  };
  if (existsSync(dir)) walk(dir);
  return out.sort();
}

interface ScannedEntry extends ManifestEntryRef {
  id: string;
  kind: KindName;
}

export async function scanPacks(dir: string): Promise<ScannedEntry[]> {
  const res: ScannedEntry[] = [];
  for (const file of listPaks(dir)) {
    const pak = '/' + relative(dir, file).split(sep).join('/');
    const src = new FileSource(file);
    try {
      const r = await FpakReader.open(src);
      for (const e of r.entries()) {
        const payload = await r.read(e.logical_id);
        res.push({
          id: e.logical_id, kind: e.kind, pak, entry: e.logical_id, offset: Number(e.offset), bytes: e.bytes,
          sha256: createHash('sha256').update(payload).digest('hex'), index_bytes: INDEX_FIXED_BYTES + Buffer.byteLength(e.logical_id),
        });
      }
    } finally {
      src.close();
    }
  }
  return res;
}

function ref(e: ScannedEntry): ManifestEntryRef {
  return { pak: e.pak, entry: e.entry, offset: e.offset, bytes: e.bytes, sha256: e.sha256, index_bytes: e.index_bytes };
}

function loadStatsByLogical(work: string): Map<string, Stats> {
  const m = new Map<string, Stats>();
  if (!existsSync(work)) return m;
  for (const d of readdirSync(work, { withFileTypes: true })) {
    const f = join(work, d.name, 'stats.json');
    if (d.isDirectory() && existsSync(f)) {
      const s = readStats(f);
      m.set(s.logical_id, s);
    }
  }
  return m;
}

function loadAudioAssetIds(work: string): Map<string, string> {
  const f = join(work, 'audio-report.json');
  const m = new Map<string, string>();
  if (!existsSync(f)) return m;
  const j = JSON.parse(readFileSync(f, 'utf8')) as { items?: { logical_id: string; asset_id?: string }[] };
  for (const i of j.items ?? []) if (i.asset_id) m.set(i.logical_id, i.asset_id);
  return m;
}

export async function buildManifest(o: BuildManifestOptions): Promise<Manifest> {
  const ledger = typeof o.ledger === 'string' ? loadLedger(o.ledger) : o.ledger;
  const stats = loadStatsByLogical(o.work);
  const audioIds = loadAudioAssetIds(o.work);
  const byBase = new Map<string, ScannedEntry[]>();
  for (const e of await scanPacks(o.packs)) {
    const base = e.id.split('@')[0] as string;
    byBase.set(base, [...(byBase.get(base) ?? []), e]);
  }
  const assets: Record<string, ManifestAsset> = {};
  for (const [base, entries] of [...byBase].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) {
    const st = stats.get(base);
    const assetId = st?.asset_id ?? audioIds.get(base);
    const row = assetId ? ledger.byAssetId.get(assetId) : undefined;
    if (row?.status === 'INTERNAL') assertStatusOk(row, base);
    const ov = Object.entries(o.overrides ?? {}).filter(([g]) => globToRegExp(g).test(base)).map(([, v]) => v);
    const merged = Object.assign({}, ...ov) as Overrides[string];
    const a: ManifestAsset = {
      kind: 'data',
      tier: merged.tier ?? [...TIERS],
      // Real only when the ledger row is OK; an explicit override cannot make a PENDING row real (checked by the gate).
      placeholder: merged.placeholder ?? !(row && row.status === 'OK'),
    };
    if (row) {
      a.license_ref = `LICENSES.md#${row.asset_id}`;
      a.req_id = merged.req_id ?? row.req_id;
    } else if (merged.req_id) a.req_id = merged.req_id;
    const sorted = [...entries].sort((x, y) => (x.id < y.id ? -1 : 1));
    for (const e of sorted) {
      const variant = e.id.includes('@') ? (e.id.split('@')[1] as string) : '';
      const lod = /^lod(\d+)(_fp|_tp)?$/.exec(variant);
      if (lod) {
        a.kind = 'model';
        const chain = st?.chains.find((c) => c.suffix === (lod[2] ?? ''));
        const tris = chain?.lods.find((l) => l.lod === Number(lod[1]))?.tris;
        (a.lods ??= []).push({ lod: Number(lod[1]), suffix: lod[2] ?? '', ...(tris !== undefined ? { tris } : {}), ...ref(e) });
      } else if (variant === 'collision') a.collision = ref(e);
      else if (variant.startsWith('tex_')) (a.textures ??= []).push({ name: variant, ...ref(e) });
      else if (variant === 'opus' || variant === 'aac') {
        a.kind = 'audio';
        (a.audio ??= []).push({ codec: variant, ...ref(e) });
      } else if (variant === 'meta') a.meta = ref(e);
      else if (e.kind === 'texture') {
        a.kind = 'texture';
        (a.textures ??= []).push({ name: variant || base, ...ref(e) });
      }
    }
    if (a.kind === 'data' && a.textures?.length) a.kind = 'texture';
    a.lods?.sort((x, y) => x.suffix.localeCompare(y.suffix) || x.lod - y.lod);
    assets[base] = a;
  }
  return {
    schema: MANIFEST_SCHEMA,
    build: o.build ?? new Date().toISOString(),
    game_version: o.gameVersion ?? '0.1.0',
    data_hash: o.dataHash ?? '',
    assets,
    bundles: o.bundles,
    budgets: o.budgets,
  };
}

/** Every entry reference of an asset (lods, collision, textures, audio, meta). */
export function assetRefs(a: ManifestAsset): ManifestEntryRef[] {
  return [...(a.lods ?? []), ...(a.collision ? [a.collision] : []), ...(a.textures ?? []), ...(a.audio ?? []), ...(a.meta ? [a.meta] : [])];
}

function sortKeys(v: unknown): unknown {
  if (Array.isArray(v)) return v.map(sortKeys);
  if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v as object).sort(([a], [b]) => (a < b ? -1 : 1)).map(([k, x]) => [k, sortKeys(x)]));
  return v;
}

export function serializeManifest(m: Manifest): string {
  return JSON.stringify(sortKeys(m), null, 2) + '\n';
}

export function writeManifest(path: string, m: Manifest): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, serializeManifest(m));
}

export function readManifest(path: string): Manifest {
  return JSON.parse(readFileSync(path, 'utf8')) as Manifest;
}

/** sha256 of the manifest without the `build` timestamp (determinism check). */
export function manifestDigest(m: Manifest): string {
  return createHash('sha256').update(serializeManifest({ ...m, build: '' })).digest('hex');
}
