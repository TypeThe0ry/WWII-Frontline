/** Client side of `manifest.json` (ARCHITECTURE 11.3; written by tools/asset-pipeline `manifest`). */
export type Tier = 'low' | 'medium' | 'high' | 'ultra';

export interface EntryRef {
  pak: string;
  entry: string;
  offset: number;
  bytes: number;
  sha256: string;
}

export interface ManifestAsset {
  kind: 'model' | 'texture' | 'audio' | 'data';
  tier: Tier[];
  placeholder: boolean;
  req_id?: string;
  lods?: (EntryRef & { lod: number; suffix: string; tris?: number })[];
  collision?: EntryRef;
  textures?: (EntryRef & { name: string })[];
  audio?: (EntryRef & { codec: 'opus' | 'aac' })[];
  meta?: EntryRef;
}

export interface Manifest {
  schema: number;
  game_version: string;
  assets: Record<string, ManifestAsset>;
}

export interface Resolved extends EntryRef {
  placeholder: boolean;
  req_id: string | undefined;
}

export class ManifestError extends Error {}

export class ManifestIndex {
  /** Logical entry id (`veh_x@lod0`, `snd_x@opus`, ...) -> reference. */
  readonly #entries = new Map<string, { ref: EntryRef; asset: ManifestAsset; base: string }>();

  constructor(readonly manifest: Manifest, readonly tier: Tier) {
    if (manifest.schema !== 1) throw new ManifestError(`unsupported manifest schema ${manifest.schema}`);
    for (const [base, a] of Object.entries(manifest.assets)) {
      if (!a.tier.includes(tier)) continue;
      const refs: EntryRef[] = [...(a.lods ?? []), ...(a.collision ? [a.collision] : []), ...(a.textures ?? []), ...(a.audio ?? []), ...(a.meta ? [a.meta] : [])];
      for (const r of refs) this.#entries.set(r.entry, { ref: r, asset: a, base });
    }
  }

  has(entryId: string): boolean {
    return this.#entries.has(entryId);
  }

  /** `resolve('veh_us_m4a1_sherman@lod0')` -> pak path, offset, bytes; undefined when absent on this tier. */
  resolve(entryId: string): Resolved | undefined {
    const e = this.#entries.get(entryId);
    return e && { ...e.ref, placeholder: e.asset.placeholder, req_id: e.asset.req_id };
  }

  /** Requirement id for a logical id (placeholder naming), if the manifest knows it. */
  reqIdOf(entryId: string): string | undefined {
    return this.#entries.get(entryId)?.asset.req_id;
  }

  /** LOD ids available for a base id, ascending. */
  lodsOf(base: string, suffix = ''): number[] {
    return (this.manifest.assets[base]?.lods ?? []).filter((l) => l.suffix === suffix).map((l) => l.lod).sort((a, b) => a - b);
  }
}

export async function loadManifest(url: string, tier: Tier, fetcher: (u: string) => Promise<Response> = (u) => fetch(u)): Promise<ManifestIndex> {
  const r = await fetcher(url);
  if (!r.ok) throw new ManifestError(`${url}: HTTP ${r.status}`);
  return new ManifestIndex((await r.json()) as Manifest, tier);
}
