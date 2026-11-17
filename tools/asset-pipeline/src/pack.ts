/**
 * `pack` (ARCHITECTURE 11.1 step 8, M1-003): `work/{asset_id}/` intermediates -> content-addressed `.fpak` files.
 *
 * Entry ids (logical ids only; no source file names, vendors or ledger ids):
 *   {logical}@lod{N}{_fp|_tp}  mesh      {logical}@collision  mesh
 *   {logical}@tex_{name}{_fp|_tp}  texture      {logical}@meta  json
 * Packs are grouped by category (the id prefix: veh, wpn, chr, ...) or by a caller supplied group key (64 m map chunks),
 * and split at `maxBytes` (default 16 MiB) between assets.
 */
import { mkdirSync, readdirSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import type { Document, Texture } from '@gltf-transform/core';
import { assertAllowed } from './allowed.ts';
import { Codec, hex } from './fpak/format.ts';
import { encodeTextureEntry, textureEntryFromKtx2 } from './fpak/entry-texture.ts';
import { encodeMeshEntry, meshEntryFromDocument } from './fpak/entry-mesh.ts';
import { FpakWriter, publishPath, type IndexEntry } from './fpak/writer.ts';
import { loadLedger, type Ledger } from './ledger.ts';
import { readGlb } from './mesh-compress.ts';
import { readStats, type Stats } from './stats.ts';

export interface PackOptions {
  work: string;
  /** Output root; packs land in `{out}/a/xx/yyyy.fpak`. */
  out: string;
  ledger: string | Ledger;
  /** Asset ids to pack; default: every `work/*` directory with a stats.json. */
  assets?: string[];
  maxBytes?: number;
  /** Group key per logical id (default: its category prefix). */
  group?: (logicalId: string, category: string) => string;
}

export interface PackedFile {
  path: string;
  file: string;
  sha256: string;
  bytes: number;
  group: string;
  entries: { logical_id: string; kind: string; offset: number; bytes: number; codec: number; sha8: string }[];
}

export interface PackResult {
  packs: PackedFile[];
  indexFile: string;
}

const safe = (s: string) => s.toLowerCase().replace(/[^a-z0-9_]+/g, '_').replace(/^_+|_+$/g, '') || 'x';

interface Pending {
  logical: string;
  entries: { id: string; kind: 'mesh' | 'texture' | 'meta'; codec: number; payload: Uint8Array }[];
}

async function buildAsset(work: string, assetId: string): Promise<Pending> {
  const dir = join(work, assetId);
  const stats: Stats = readStats(join(dir, 'stats.json'));
  const logical = stats.logical_id;
  const entries: Pending['entries'] = [];
  const multi = stats.chains.length > 1;
  const texIds = new Map<string, string>();
  for (const chain of stats.chains) {
    const docs: Document[] = [];
    for (const l of chain.lods) docs.push(await readGlb(join(dir, l.file)));
    const first = docs[0] as Document;
    const idOf = (t: Texture): string => {
      const name = t.getName() || t.getURI() || 'texture';
      return `${logical}@tex_${safe(name)}${multi ? chain.suffix : ''}`;
    };
    for (const t of first.getRoot().listTextures()) {
      const id = idOf(t);
      if (texIds.has(id)) continue;
      texIds.set(id, id);
      const img = t.getImage();
      if (!img) continue;
      entries.push({ id, kind: 'texture', codec: Codec.textureV1, payload: encodeTextureEntry(textureEntryFromKtx2(img)) });
    }
    for (let i = 0; i < docs.length; i++) {
      const lod = chain.lods[i]!;
      const e = meshEntryFromDocument(docs[i] as Document, { textureId: idOf });
      entries.push({ id: `${logical}@lod${lod.lod}${chain.suffix}`, kind: 'mesh', codec: Codec.meshV1, payload: await encodeMeshEntry(e) });
    }
  }
  if (stats.collision) {
    const doc = await readGlb(join(dir, stats.collision.file));
    entries.push({ id: `${logical}@collision`, kind: 'mesh', codec: Codec.meshV1, payload: await encodeMeshEntry(meshEntryFromDocument(doc, { textureId: () => '' })) });
  }
  const meta = {
    logical_id: logical,
    chains: stats.chains.map((c) => ({ suffix: c.suffix, lods: c.lods.map((l) => ({ lod: l.lod, tris: l.tris, vertices: l.vertices })) })),
    collision: stats.collision ? { mode: stats.collision.mode, tris: stats.collision.tris, hulls: stats.collision.hulls } : null,
    textures: stats.textures.map((t) => ({ slot: t.slot, format: t.format, width: t.width, height: t.height, levels: t.levels })),
  };
  entries.push({ id: `${logical}@meta`, kind: 'meta', codec: Codec.json, payload: new TextEncoder().encode(JSON.stringify(meta)) });
  return { logical, entries };
}

export async function pack(o: PackOptions): Promise<PackResult> {
  const ledger = typeof o.ledger === 'string' ? loadLedger(o.ledger) : o.ledger;
  const ids = o.assets ?? readdirSync(o.work, { withFileTypes: true }).filter((d) => d.isDirectory() && existsSync(join(o.work, d.name, 'stats.json'))).map((d) => d.name).sort();
  const maxBytes = o.maxBytes ?? 16 * 1024 * 1024;
  const groups = new Map<string, Pending[]>();
  for (const id of ids) {
    assertAllowed(id, 'CONVERT', ledger); // ledger row must exist and be OK; processed assets all passed CONVERT in normalize
    const p = await buildAsset(o.work, id);
    const category = p.logical.split('_')[0] as string;
    const g = o.group ? o.group(p.logical, category) : category;
    groups.set(g, [...(groups.get(g) ?? []), p]);
  }
  const packs: PackedFile[] = [];
  const flush = (group: string, w: FpakWriter) => {
    if (w.count === 0) return;
    const b = w.finish();
    const pubPath = publishPath(b.sha256);
    const file = join(o.out, pubPath.slice(1));
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, b.bytes);
    packs.push({ path: pubPath, file, sha256: b.sha256, bytes: b.bytes.byteLength, group, entries: b.index.map(entryJson) });
  };
  for (const [group, assets] of [...groups].sort(([a], [b]) => a.localeCompare(b))) {
    let w = new FpakWriter();
    for (const a of assets) {
      const size = a.entries.reduce((s, e) => s + e.payload.byteLength, 0);
      if (w.count > 0 && w.size + size > maxBytes) {
        flush(group, w);
        w = new FpakWriter();
      }
      for (const e of a.entries) w.add(e.id, e.kind, e.codec, e.payload);
    }
    flush(group, w);
  }
  mkdirSync(o.out, { recursive: true });
  const indexFile = join(o.out, 'packs.json');
  writeFileSync(indexFile, JSON.stringify({ schema: 'frontline.packs/1', packs: packs.map((p) => ({ path: p.path, sha256: p.sha256, bytes: p.bytes, group: p.group, entries: p.entries })) }, null, 2) + '\n');
  return { packs, indexFile };
}

function entryJson(e: IndexEntry): PackedFile['entries'][number] {
  return { logical_id: e.logical_id, kind: e.kind, offset: Number(e.offset), bytes: e.bytes, codec: e.codec, sha8: hex(e.sha8) };
}
