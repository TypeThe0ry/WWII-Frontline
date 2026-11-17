/**
 * `process` (ARCHITECTURE 11.1 steps 3-6, M1-002): normalized.gltf -> textures (KTX2) -> LOD chains -> mesh compression ->
 * collision -> `work/{asset_id}/lod{N}{suffix}.glb`, `collision.glb`, `stats.json`. All outputs are intermediate products.
 *
 * Permissions (M1-001 `assertAllowed`): LOD for every mesh step (LOD chains, mesh compression, collision), TEXCOMP before any
 * texture is touched. A weapon with `variants` / `first_person` yields independent `_fp` / `_tp` chains.
 */
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { NodeIO } from '@gltf-transform/core';
import { assertAllowed, setDefaultLedger } from './allowed.ts';
import { chainsOf, type AssetClass } from './asset-class.ts';
import { simplifyCollision, type CollisionResult } from './collision.ts';
import { ExitCode, GateError } from './errors.ts';
import { loadLedger, type Ledger } from './ledger.ts';
import { generateLods } from './lod.ts';
import { compressMesh, writeGlb } from './mesh-compress.ts';
import { writeStats, type ChainStat, type CollisionStat, type Stats } from './stats.ts';
import { processTextures, type TexResult } from './textures.ts';
import { assertLogicalId } from './naming.ts';
import { KHRTextureBasisu, EXTMeshoptCompression, KHRMeshQuantization } from '@gltf-transform/extensions';

export interface ProcessOptions {
  work: string;
  ledger: string | Ledger;
  assetId: string;
  logicalId: string;
  cls: AssetClass;
  /** Skip the collision step even when `cls.collision` is set. */
  noCollision?: boolean;
}

export interface ProcessResult {
  stats: Stats;
  statsFile: string;
  collision?: CollisionResult;
  textures: TexResult[];
  lods: ChainStat['lods'];
}

async function sourceBytes(gltfPath: string): Promise<number> {
  const json = await new NodeIO().readAsJSON(gltfPath);
  let n = Buffer.byteLength(JSON.stringify(json.json));
  for (const r of Object.values(json.resources)) n += r.byteLength;
  return n;
}

export async function processMesh(o: ProcessOptions): Promise<ProcessResult> {
  assertLogicalId(o.logicalId);
  const ledger = typeof o.ledger === 'string' ? loadLedger(o.ledger) : o.ledger;
  setDefaultLedger(ledger);
  assertAllowed(o.assetId, 'LOD', ledger);
  const dir = join(o.work, o.assetId);
  const warnings: string[] = [];
  const io = new NodeIO().registerExtensions([KHRTextureBasisu, EXTMeshoptCompression, KHRMeshQuantization]);
  const chains: ChainStat[] = [];
  let textures: TexResult[] = [];
  let source = 0;
  let firstDoc: Awaited<ReturnType<NodeIO['read']>> | undefined;

  for (const chain of chainsOf(o.cls)) {
    const rel = chain.input ?? 'normalized.gltf';
    const input = join(dir, rel);
    if (!existsSync(input)) throw new GateError(ExitCode.Usage, `${input}: not found (run normalize first)`);
    source = Math.max(source, await sourceBytes(input));
    const doc = await io.read(input);
    // Textures are encoded once per chain input, before the LOD clones are made (every LOD shares the KTX2 images).
    const tex = await processTextures(doc, { cls: o.cls, assetId: o.assetId, ledger });
    warnings.push(...tex.warnings.map((w) => `${chain.suffix || 'base'}: ${w}`));
    if (!chain.suffix || chains.length === 0) textures = tex.textures;
    const levels = await generateLods(doc, chain.lodRatios);
    firstDoc ??= doc;
    const lods: ChainStat['lods'] = [];
    let err = 0;
    for (const lv of levels) {
      const c = await compressMesh(lv.doc);
      err = Math.max(err, c.bboxError);
      const file = `lod${lv.lod}${chain.suffix}.glb`;
      const bytes = await writeGlb(lv.doc, join(dir, file));
      lods.push({
        lod: lv.lod, tris: lv.tris, vertices: lv.vertices, bytes,
        simplify_error: lv.simplifyError, simplify_error_m: lv.simplifyErrorAbs, seam_lock: lv.seamLock, file,
      });
    }
    chains.push({ suffix: chain.suffix, lods, bbox_error: err });
  }

  let collision: CollisionResult | undefined;
  let cstat: CollisionStat | undefined;
  if (o.cls.collision && !o.noCollision && firstDoc) {
    const c = o.cls.collision;
    collision = await simplifyCollision(firstDoc, {
      mode: c.mode,
      ...(c.max_tris !== undefined ? { maxTris: c.max_tris } : {}),
      ...(c.max_hulls !== undefined ? { maxHulls: c.max_hulls } : {}),
      ...(c.max_hull_vertices !== undefined ? { maxHullVertices: c.max_hull_vertices } : {}),
      ...(o.cls.class ? { kind: o.cls.class } : {}),
    });
    const bytes = await writeGlb(collision.doc, join(dir, 'collision.glb'));
    cstat = { mode: collision.mode, tris: collision.tris, hulls: collision.hulls, source_tris: collision.sourceTris, bytes, file: 'collision.glb' };
  }

  const stats: Stats = {
    schema: 'frontline.asset-stats/1',
    asset_id: o.assetId,
    logical_id: o.logicalId,
    source_bytes: source,
    lods: (chains[0] as ChainStat).lods,
    chains,
    ...(cstat ? { collision: cstat } : {}),
    textures,
    warnings,
  };
  const statsFile = join(dir, 'stats.json');
  writeStats(statsFile, stats);
  return { stats, statsFile, ...(collision ? { collision } : {}), textures, lods: stats.lods };
}

/** Load `asset-class.json` text if present (helper for the CLI). */
export function readJsonFile(path: string): unknown {
  return JSON.parse(readFileSync(path, 'utf8'));
}
