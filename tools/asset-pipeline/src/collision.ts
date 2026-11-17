/**
 * Collision simplification (ARCHITECTURE 11.1 step 6, M1-002).
 *  - `trimesh`: weld + meshoptimizer simplify of the whole static geometry down to <= maxTris triangles.
 *  - `convex`: split the triangle cloud into <= maxHulls clusters (recursive median split along the longest axis) and
 *    build one convex hull per cluster (a V-HACD-like approximation without concavity refinement; no new dependency).
 * Output documents carry positions + indices only: no UVs, normals or materials.
 */
import { Document, type Node } from '@gltf-transform/core';
import { MeshoptSimplifier } from 'meshoptimizer';
import { DEFAULT_COLLISION_TRIS, MAX_HULLS, type AssetKind } from './asset-class.ts';

export interface CollisionOptions {
  mode: 'trimesh' | 'convex';
  maxTris?: number;
  maxHulls?: number;
  maxHullVertices?: number;
  kind?: AssetKind;
}

export interface CollisionResult {
  mode: 'trimesh' | 'convex';
  doc: Document;
  /** Triangles in the output (all hulls summed in convex mode). */
  tris: number;
  /** Convex pieces (convex mode) or 1. */
  hulls: number;
  sourceTris: number;
  /** meshoptimizer error relative to the extent (trimesh), 0 otherwise. */
  simplifyError: number;
}

interface Soup {
  positions: Float32Array;
  indices: Uint32Array;
}

type M4 = readonly number[];

function applyM4(m: M4, x: number, y: number, z: number): [number, number, number] {
  return [
    (m[0] as number) * x + (m[4] as number) * y + (m[8] as number) * z + (m[12] as number),
    (m[1] as number) * x + (m[5] as number) * y + (m[9] as number) * z + (m[13] as number),
    (m[2] as number) * x + (m[6] as number) * y + (m[10] as number) * z + (m[14] as number),
  ];
}

/** All triangle geometry of the default scene, baked into world space and welded by exact position. */
export function collectSoup(doc: Document): Soup {
  const pos: number[] = [];
  const idx: number[] = [];
  const weld = new Map<string, number>();
  const visit = (node: Node) => {
    const mesh = node.getMesh();
    if (mesh) {
      const world = node.getWorldMatrix();
      for (const p of mesh.listPrimitives()) {
        if (p.getMode() !== 4) continue;
        const a = p.getAttribute('POSITION');
        if (!a) continue;
        const map: number[] = [];
        const v = [0, 0, 0];
        for (let i = 0; i < a.getCount(); i++) {
          a.getElement(i, v);
          const [x, y, z] = applyM4(world, v[0] as number, v[1] as number, v[2] as number);
          const key = `${Math.fround(x)},${Math.fround(y)},${Math.fround(z)}`;
          let id = weld.get(key);
          if (id === undefined) {
            id = pos.length / 3;
            weld.set(key, id);
            pos.push(Math.fround(x), Math.fround(y), Math.fround(z));
          }
          map.push(id);
        }
        const ind = p.getIndices();
        const n = ind ? ind.getCount() : a.getCount();
        for (let t = 0; t + 2 < n; t += 3) {
          const i0 = map[ind ? (ind.getScalar(t) as number) : t] as number;
          const i1 = map[ind ? (ind.getScalar(t + 1) as number) : t + 1] as number;
          const i2 = map[ind ? (ind.getScalar(t + 2) as number) : t + 2] as number;
          if (i0 !== i1 && i1 !== i2 && i0 !== i2) idx.push(i0, i1, i2);
        }
      }
    }
    for (const c of node.listChildren()) visit(c);
  };
  const scene = doc.getRoot().getDefaultScene() ?? doc.getRoot().listScenes()[0];
  if (scene) for (const n of scene.listChildren()) visit(n);
  return { positions: new Float32Array(pos), indices: Uint32Array.from(idx) };
}

function buildDoc(meshes: { positions: Float32Array; indices: Uint32Array; name: string }[]): Document {
  const doc = new Document();
  const buffer = doc.createBuffer('collision');
  const scene = doc.createScene('collision');
  for (const m of meshes) {
    const prim = doc
      .createPrimitive()
      .setAttribute('POSITION', doc.createAccessor(`${m.name}_pos`).setType('VEC3').setArray(m.positions).setBuffer(buffer))
      .setIndices(doc.createAccessor(`${m.name}_idx`).setType('SCALAR').setArray(m.indices).setBuffer(buffer));
    const mesh = doc.createMesh(m.name).addPrimitive(prim);
    scene.addChild(doc.createNode(m.name).setMesh(mesh));
  }
  return doc;
}

/** Compact a soup to the vertices that are referenced. */
function compact(positions: Float32Array, indices: Uint32Array): Soup {
  const remap = new Map<number, number>();
  const out: number[] = [];
  const idx = new Uint32Array(indices.length);
  for (let i = 0; i < indices.length; i++) {
    const v = indices[i] as number;
    let n = remap.get(v);
    if (n === undefined) {
      n = remap.size;
      remap.set(v, n);
      out.push(positions[v * 3] as number, positions[v * 3 + 1] as number, positions[v * 3 + 2] as number);
    }
    idx[i] = n;
  }
  return { positions: new Float32Array(out), indices: idx };
}

async function simplifyTrimesh(soup: Soup, maxTris: number): Promise<{ soup: Soup; error: number }> {
  await MeshoptSimplifier.ready;
  const scale = MeshoptSimplifier.getScale(soup.positions, 3);
  const target = Math.max(1, Math.floor(maxTris)) * 3;
  if (soup.indices.length <= target) return { soup: compact(soup.positions, soup.indices), error: 0 };
  // Precise simplification first (collapse and prune), sloppy as the fallback that always honours the budget.
  let [ind, err] = MeshoptSimplifier.simplify(soup.indices, soup.positions, 3, target, 1, ['Prune']);
  if (ind.length > target) [ind, err] = MeshoptSimplifier.simplifySloppy(soup.indices, soup.positions, 3, null, target, 1);
  return { soup: compact(soup.positions, ind), error: scale > 0 ? err : 0 };
}

// ---- convex hull (incremental) -------------------------------------------------------------------------------------

type V3 = [number, number, number];
const sub = (a: V3, b: V3): V3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const cross = (a: V3, b: V3): V3 => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const at = (v: V3, k: number): number => v[k] as number;
const dot = (a: V3, b: V3): number => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

/** Convex hull of a point set; returns outward-wound triangles over `pts` indices, or null when the set is degenerate. */
export function convexHull(pts: V3[]): [number, number, number][] | null {
  const n = pts.length;
  if (n < 4) return null;
  let diag = 0;
  for (let k = 0; k < 3; k++) {
    let lo = Infinity;
    let hi = -Infinity;
    for (const p of pts) {
      lo = Math.min(lo, at(p, k));
      hi = Math.max(hi, at(p, k));
    }
    diag = Math.max(diag, hi - lo);
  }
  const eps = 1e-9 * Math.max(diag, 1e-9);
  // Initial tetrahedron: farthest pair, farthest from that line, farthest from that plane.
  const i0 = 0;
  let i1 = -1;
  let best = -1;
  for (let i = 1; i < n; i++) {
    const d = dot(sub(pts[i] as V3, pts[0] as V3), sub(pts[i] as V3, pts[0] as V3));
    if (d > best) {
      best = d;
      i1 = i;
    }
  }
  if (i1 < 0 || best <= eps * eps) return null;
  let i2 = -1;
  best = -1;
  for (let i = 0; i < n; i++) {
    const c = cross(sub(pts[i1] as V3, pts[i0] as V3), sub(pts[i] as V3, pts[i0] as V3));
    const d = dot(c, c);
    if (d > best) {
      best = d;
      i2 = i;
    }
  }
  if (i2 < 0 || best <= eps * eps * diag * diag) return null;
  let i3 = -1;
  best = -1;
  const nrm = cross(sub(pts[i1] as V3, pts[i0] as V3), sub(pts[i2] as V3, pts[i0] as V3));
  for (let i = 0; i < n; i++) {
    const d = Math.abs(dot(nrm, sub(pts[i] as V3, pts[i0] as V3)));
    if (d > best) {
      best = d;
      i3 = i;
    }
  }
  if (i3 < 0 || best <= eps * diag * diag) return null;
  const side = dot(nrm, sub(pts[i3] as V3, pts[i0] as V3));
  let faces: [number, number, number][] =
    side > 0
      ? [
          [i0, i2, i1],
          [i0, i1, i3],
          [i1, i2, i3],
          [i2, i0, i3],
        ]
      : [
          [i0, i1, i2],
          [i0, i3, i1],
          [i1, i3, i2],
          [i2, i3, i0],
        ];
  const outside = (f: [number, number, number], p: V3): number => {
    const a = pts[f[0]] as V3;
    return dot(cross(sub(pts[f[1]] as V3, a), sub(pts[f[2]] as V3, a)), sub(p, a));
  };
  for (let i = 0; i < n; i++) {
    if (i === i0 || i === i1 || i === i2 || i === i3) continue;
    const p = pts[i] as V3;
    const visible = faces.map((f) => outside(f, p) > eps * diag * diag);
    if (!visible.some(Boolean)) continue;
    const edges = new Map<string, [number, number]>();
    faces.forEach((f, fi) => {
      if (!visible[fi]) return;
      for (let e = 0; e < 3; e++) {
        const a = f[e] as number;
        const b = f[(e + 1) % 3] as number;
        const rev = `${b},${a}`;
        if (edges.has(rev)) edges.delete(rev);
        else edges.set(`${a},${b}`, [a, b]);
      }
    });
    faces = faces.filter((_, fi) => !visible[fi]);
    for (const [a, b] of edges.values()) faces.push([a, b, i]);
  }
  return faces;
}

/** Reduce a cluster to at most `maxPts` points: extreme points along a fixed set of directions. */
function extremePoints(pts: V3[], maxPts: number): V3[] {
  if (pts.length <= maxPts) return pts;
  const dirs: V3[] = [];
  for (const x of [-1, 0, 1]) for (const y of [-1, 0, 1]) for (const z of [-1, 0, 1]) if (x || y || z) dirs.push([x, y, z]);
  const picked = new Set<number>();
  for (const d of dirs) {
    let bi = 0;
    let bv = -Infinity;
    pts.forEach((p, i) => {
      const v = dot(p, d);
      if (v > bv) {
        bv = v;
        bi = i;
      }
    });
    picked.add(bi);
  }
  // Top up with the points farthest from the centroid until the budget is reached.
  const c: V3 = [0, 0, 0];
  for (const p of pts) for (let k = 0; k < 3; k++) c[k] = at(c, k) + at(p, k) / pts.length;
  const rest = pts
    .map((p, i) => ({ i, d: dot(sub(p, c), sub(p, c)) }))
    .filter((e) => !picked.has(e.i))
    .sort((a, b) => b.d - a.d);
  for (const e of rest) {
    if (picked.size >= maxPts) break;
    picked.add(e.i);
  }
  return [...picked].map((i) => pts[i] as V3);
}

function boxHull(pts: V3[]): { positions: Float32Array; indices: Uint32Array } {
  const lo: V3 = [Infinity, Infinity, Infinity];
  const hi: V3 = [-Infinity, -Infinity, -Infinity];
  for (const p of pts) for (let k = 0; k < 3; k++) {
    lo[k] = Math.min(at(lo, k), at(p, k));
    hi[k] = Math.max(at(hi, k), at(p, k));
  }
  const positions: number[] = [];
  for (let i = 0; i < 8; i++) positions.push(i & 1 ? hi[0] : lo[0], i & 2 ? hi[1] : lo[1], i & 4 ? hi[2] : lo[2]);
  const q = (a: number, b: number, c: number, d: number) => [a, b, c, a, c, d];
  const indices = [...q(0, 4, 6, 2), ...q(1, 3, 7, 5), ...q(0, 1, 5, 4), ...q(2, 6, 7, 3), ...q(0, 2, 3, 1), ...q(4, 5, 7, 6)];
  return { positions: Float32Array.from(positions), indices: Uint32Array.from(indices) };
}

function splitClusters(tris: number[], centroid: (t: number) => V3, maxClusters: number): number[][] {
  let clusters: number[][] = [tris];
  while (clusters.length < maxClusters) {
    // Split the cluster with the most triangles.
    let bi = 0;
    for (let i = 1; i < clusters.length; i++) if ((clusters[i] as number[]).length > (clusters[bi] as number[]).length) bi = i;
    const c = clusters[bi] as number[];
    if (c.length < 2) break;
    const lo: V3 = [Infinity, Infinity, Infinity];
    const hi: V3 = [-Infinity, -Infinity, -Infinity];
    for (const t of c) {
      const p = centroid(t);
      for (let k = 0; k < 3; k++) {
        lo[k] = Math.min(at(lo, k), at(p, k));
        hi[k] = Math.max(at(hi, k), at(p, k));
      }
    }
    let axis = 0;
    for (let k = 1; k < 3; k++) if (at(hi, k) - at(lo, k) > at(hi, axis) - at(lo, axis)) axis = k;
    const sorted = [...c].sort((a, b) => at(centroid(a), axis) - at(centroid(b), axis));
    const half = sorted.length >> 1;
    clusters = clusters.filter((_, i) => i !== bi);
    clusters.push(sorted.slice(0, half), sorted.slice(half));
  }
  return clusters;
}

function convexPieces(soup: Soup, maxHulls: number, maxHullVertices: number): { positions: Float32Array; indices: Uint32Array }[] {
  const nt = soup.indices.length / 3;
  const P = soup.positions;
  const vert = (i: number): V3 => [P[i * 3] as number, P[i * 3 + 1] as number, P[i * 3 + 2] as number];
  const centroid = (t: number): V3 => {
    const a = vert(soup.indices[t * 3] as number);
    const b = vert(soup.indices[t * 3 + 1] as number);
    const c = vert(soup.indices[t * 3 + 2] as number);
    return [(a[0] + b[0] + c[0]) / 3, (a[1] + b[1] + c[1]) / 3, (a[2] + b[2] + c[2]) / 3];
  };
  const clusters = splitClusters(Array.from({ length: nt }, (_, i) => i), centroid, Math.min(maxHulls, MAX_HULLS));
  const out: { positions: Float32Array; indices: Uint32Array }[] = [];
  for (const c of clusters) {
    const uniq = new Map<number, V3>();
    for (const t of c) for (let k = 0; k < 3; k++) {
      const v = soup.indices[t * 3 + k] as number;
      uniq.set(v, vert(v));
    }
    const pts = extremePoints([...uniq.values()], maxHullVertices);
    const faces = convexHull(pts);
    if (!faces) {
      out.push(boxHull([...uniq.values()]));
      continue;
    }
    const positions = new Float32Array(pts.length * 3);
    pts.forEach((p, i) => positions.set(p, i * 3));
    const hull = compact(positions, Uint32Array.from(faces.flat()));
    out.push(hull);
  }
  return out;
}

/** Simplified collision geometry of `doc` (the visual document is not modified). */
export async function simplifyCollision(doc: Document, o: CollisionOptions): Promise<CollisionResult> {
  const soup = collectSoup(doc);
  const sourceTris = soup.indices.length / 3;
  if (sourceTris === 0) throw new Error('simplifyCollision: the document has no triangles');
  if (o.mode === 'trimesh') {
    const maxTris = o.maxTris ?? DEFAULT_COLLISION_TRIS[o.kind ?? 'prop'];
    const { soup: s, error } = await simplifyTrimesh(soup, maxTris);
    return { mode: 'trimesh', doc: buildDoc([{ ...s, name: 'collision' }]), tris: s.indices.length / 3, hulls: 1, sourceTris, simplifyError: error };
  }
  const maxHulls = Math.min(o.maxHulls ?? MAX_HULLS, MAX_HULLS);
  const pieces = convexPieces(soup, maxHulls, o.maxHullVertices ?? 48);
  return {
    mode: 'convex',
    doc: buildDoc(pieces.map((p, i) => ({ ...p, name: `hull_${i}` }))),
    tris: pieces.reduce((s, p) => s + p.indices.length / 3, 0),
    hulls: pieces.length,
    sourceTris,
    simplifyError: 0,
  };
}
