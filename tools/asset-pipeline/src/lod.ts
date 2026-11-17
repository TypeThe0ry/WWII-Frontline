/**
 * LOD generation (ARCHITECTURE 11.1 step 3, M1-002): meshoptimizer simplify with UV-seam protection.
 *
 * Every LOD level is simplified independently from LOD0 (not cascaded). Per primitive:
 *  - vertices that share a position but carry different UVs (UV seams / atlas island borders) are locked through
 *    `vertex_lock`, together with `LockBorder` (primitive borders = material boundaries). Both keep the silhouette of
 *    texture islands intact, so the texture never swims at a seam.
 *  - normals and UVs also enter the error metric as weighted attributes.
 *  - if hard-locking the seams prevents reaching the ratio (atlas models with many islands), the level is retried with
 *    seams only weighted (still LockBorder) and the result is flagged `relaxed`; the ratio is never silently loosened.
 */
import { Document, type Primitive } from '@gltf-transform/core';
import { cloneDocument, compactPrimitive } from '@gltf-transform/functions';
import { MeshoptSimplifier } from 'meshoptimizer';

export type SeamLock = 'none' | 'hard' | 'relaxed';

export interface LodLevel {
  lod: number;
  ratio: number;
  doc: Document;
  tris: number;
  vertices: number;
  /** meshoptimizer error of the worst primitive, relative to the primitive extent (0 for LOD0). */
  simplifyError: number;
  /** Same error in model units (metres): relative error x extent. */
  simplifyErrorAbs: number;
  seamLock: SeamLock;
}

export interface LodOptions {
  /** Relative error cap handed to meshoptimizer; high on purpose so that the ratio is what limits the result. */
  targetError?: number;
  lockSeams?: boolean;
  normalWeight?: number;
  uvWeight?: number;
}

const ATTR_N = 3;
const ATTR_UV = 2;

function floatArray(acc: ReturnType<Primitive['getAttribute']>, n: number): Float32Array {
  const count = acc?.getCount() ?? 0;
  const out = new Float32Array(count * n);
  if (!acc) return out;
  const v: number[] = new Array(acc.getElementSize()).fill(0);
  for (let i = 0; i < count; i++) {
    acc.getElement(i, v);
    for (let k = 0; k < n; k++) out[i * n + k] = v[k] ?? 0;
  }
  return out;
}

function triangleIndices(prim: Primitive): Uint32Array {
  const idx = prim.getIndices();
  if (idx) return Uint32Array.from(idx.getArray() as ArrayLike<number>);
  const n = prim.getAttribute('POSITION')?.getCount() ?? 0;
  return Uint32Array.from({ length: n }, (_, i) => i);
}

function posKey(p: Float32Array, i: number): string {
  return `${p[i * 3]},${p[i * 3 + 1]},${p[i * 3 + 2]}`;
}

/** Vertices that share a position with another vertex but have a different UV (seam members, all wedges of that position). */
export function findSeamVertices(positions: Float32Array, uvs: Float32Array | null, eps = 1e-5): Uint8Array {
  const n = positions.length / 3;
  const lock = new Uint8Array(n);
  if (!uvs) return lock;
  const groups = new Map<string, number[]>();
  for (let i = 0; i < n; i++) {
    const k = posKey(positions, i);
    const g = groups.get(k);
    if (g) g.push(i);
    else groups.set(k, [i]);
  }
  for (const g of groups.values()) {
    if (g.length < 2) continue;
    const first = g[0] as number;
    const seam = g.some((i) => Math.abs((uvs[i * 2] as number) - (uvs[first * 2] as number)) > eps || Math.abs((uvs[i * 2 + 1] as number) - (uvs[first * 2 + 1] as number)) > eps);
    if (seam) for (const i of g) lock[i] = 1;
  }
  return lock;
}

/** Position keys (exact float32 bits) of the seam vertices of a primitive. */
export function seamPositionKeys(prim: Primitive): Set<string> {
  const pos = floatArray(prim.getAttribute('POSITION'), 3);
  const uv = prim.getAttribute('TEXCOORD_0');
  const lock = findSeamVertices(pos, uv ? floatArray(uv, 2) : null);
  const keys = new Set<string>();
  for (let i = 0; i < lock.length; i++) if (lock[i]) keys.add(posKey(pos, i));
  return keys;
}

/** Fraction of the seam positions of `before` that still exist as a vertex of the same-index primitive in `after`. */
export function seamRetention(before: Document, after: Document): { seamVertices: number; retained: number; ratio: number } {
  const b = before.getRoot().listMeshes().flatMap((m) => m.listPrimitives());
  const a = after.getRoot().listMeshes().flatMap((m) => m.listPrimitives());
  let total = 0;
  let kept = 0;
  b.forEach((p, i) => {
    const keys = seamPositionKeys(p);
    total += keys.size;
    const q = a[i];
    if (!q) return;
    const pos = floatArray(q.getAttribute('POSITION'), 3);
    const present = new Set<string>();
    for (let v = 0; v < pos.length / 3; v++) present.add(posKey(pos, v));
    for (const k of keys) if (present.has(k)) kept++;
  });
  return { seamVertices: total, retained: kept, ratio: total === 0 ? 1 : kept / total };
}

interface SimplifyOutcome {
  tris: number;
  error: number;
  errorAbs: number;
  lock: SeamLock;
}

function simplifyPrimitive(prim: Primitive, ratio: number, opts: Required<LodOptions>): SimplifyOutcome {
  const indices = triangleIndices(prim);
  const srcTris = indices.length / 3;
  const pos = floatArray(prim.getAttribute('POSITION'), 3);
  const nrmAcc = prim.getAttribute('NORMAL');
  const uvAcc = prim.getAttribute('TEXCOORD_0');
  const nrm = nrmAcc ? floatArray(nrmAcc, ATTR_N) : null;
  const uv = uvAcc ? floatArray(uvAcc, ATTR_UV) : null;
  const vcount = pos.length / 3;
  const stride = (nrm ? ATTR_N : 0) + (uv ? ATTR_UV : 0);
  const attrs = new Float32Array(vcount * stride);
  const weights: number[] = [];
  if (nrm) for (let k = 0; k < ATTR_N; k++) weights.push(opts.normalWeight);
  if (uv) for (let k = 0; k < ATTR_UV; k++) weights.push(opts.uvWeight);
  for (let i = 0; i < vcount; i++) {
    let o = i * stride;
    if (nrm) for (let k = 0; k < ATTR_N; k++) attrs[o++] = nrm[i * ATTR_N + k] as number;
    if (uv) for (let k = 0; k < ATTR_UV; k++) attrs[o++] = uv[i * ATTR_UV + k] as number;
  }
  const scale = MeshoptSimplifier.getScale(pos, 3);
  const target = Math.max(1, Math.round(srcTris * ratio)) * 3;
  const run = (lock: Uint8Array | null): [Uint32Array, number] =>
    stride > 0
      ? MeshoptSimplifier.simplifyWithAttributes(indices, pos, 3, attrs, stride, weights, lock, target, opts.targetError, ['LockBorder'])
      : lock
        ? MeshoptSimplifier.simplifyWithAttributes(indices, pos, 3, new Float32Array(0), 0, [], lock, target, opts.targetError, ['LockBorder'])
        : MeshoptSimplifier.simplify(indices, pos, 3, target, opts.targetError, ['LockBorder']);

  let mode: SeamLock = 'none';
  let result: [Uint32Array, number];
  const seams = opts.lockSeams ? findSeamVertices(pos, uv) : null;
  const hasSeams = seams !== null && seams.some((x) => x === 1);
  if (hasSeams) {
    mode = 'hard';
    result = run(seams);
    // Allow 5 % over target (card: ratios within +-5 %); beyond that fall back to weighted-only seams.
    if (result[0].length / 3 > (target / 3) * 1.05 + 1) {
      const relaxed = run(null);
      if (relaxed[0].length < result[0].length) {
        result = relaxed;
        mode = 'relaxed';
      }
    }
  } else {
    result = run(null);
  }
  const [out, err] = result;
  const oldAcc = prim.getIndices();
  const arr = vcount <= 65535 ? Uint16Array.from(out) : out;
  const doc = Document.fromGraph(prim.getGraph()) as Document;
  const fresh = doc.createAccessor('indices').setType('SCALAR').setArray(arr);
  const buf = oldAcc?.getBuffer();
  if (buf) fresh.setBuffer(buf);
  prim.setIndices(fresh);
  if (oldAcc && oldAcc.listParents().every((x) => x.propertyType === 'Root')) oldAcc.dispose();
  compactPrimitive(prim);
  return { tris: out.length / 3, error: err, errorAbs: err * scale, lock: mode };
}

export function countTriangles(doc: Document): { tris: number; vertices: number } {
  let tris = 0;
  let vertices = 0;
  for (const m of doc.getRoot().listMeshes()) {
    for (const p of m.listPrimitives()) {
      if (p.getMode() !== 4) continue;
      tris += (p.getIndices()?.getCount() ?? p.getAttribute('POSITION')?.getCount() ?? 0) / 3;
      vertices += p.getAttribute('POSITION')?.getCount() ?? 0;
    }
  }
  return { tris, vertices };
}

/** Make sure every triangle primitive has an index accessor (simplify needs one); no-op after normalize. */
function ensureIndexed(doc: Document): void {
  const buffer = doc.getRoot().listBuffers()[0] ?? doc.createBuffer('lod');
  for (const m of doc.getRoot().listMeshes()) {
    for (const p of m.listPrimitives()) {
      if (p.getMode() !== 4 || p.getIndices()) continue;
      const n = p.getAttribute('POSITION')?.getCount() ?? 0;
      const idx = Uint32Array.from({ length: n }, (_, i) => i);
      p.setIndices(doc.createAccessor('indices').setType('SCALAR').setArray(idx).setBuffer(buffer));
    }
  }
}

/**
 * Generate LOD0..LODn documents from `source` (left untouched). `ratios[0]` must be 1.
 * Needs `await MeshoptSimplifier.ready` (done here).
 */
export async function generateLods(source: Document, ratios: readonly number[] = [1, 0.5, 0.25, 0.1], options: LodOptions = {}): Promise<LodLevel[]> {
  await MeshoptSimplifier.ready;
  const opts: Required<LodOptions> = { targetError: 1, lockSeams: true, normalWeight: 0.5, uvWeight: 1, ...options };
  const levels: LodLevel[] = [];
  for (let lod = 0; lod < ratios.length; lod++) {
    const ratio = ratios[lod] as number;
    const doc = cloneDocument(source);
    ensureIndexed(doc);
    let err = 0;
    let errAbs = 0;
    let lockMode: SeamLock = 'none';
    if (ratio < 1) {
      for (const m of doc.getRoot().listMeshes()) {
        for (const p of m.listPrimitives()) {
          if (p.getMode() !== 4) continue;
          const r = simplifyPrimitive(p, ratio, opts);
          err = Math.max(err, r.error);
          errAbs = Math.max(errAbs, r.errorAbs);
          if (r.lock === 'relaxed' || (r.lock === 'hard' && lockMode === 'none')) lockMode = r.lock;
        }
      }
    }
    const c = countTriangles(doc);
    levels.push({ lod, ratio, doc, tris: c.tris, vertices: c.vertices, simplifyError: err, simplifyErrorAbs: errAbs, seamLock: lockMode });
  }
  return levels;
}
