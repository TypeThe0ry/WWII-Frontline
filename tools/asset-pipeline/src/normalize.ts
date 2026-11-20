/**
 * normalize (ARCHITECTURE §11.1 step 2): unit -> metres, axes -> Y-up / -Z-forward, origin reset, triangulation and
 * merging of same-material sub-meshes. Output `work/{asset_id}/normalized.gltf` is an intermediate product only.
 *
 * Source declarations (all optional; CLI flags / options win):
 *   glTF `asset.extras` (or top-level `extras`):  { "unit": "cm" | "m" | "mm" | "in" | "ft" | <metres per unit>,
 *                                                  "up_axis": "Y" | "Z", "forward_axis": "+Z" | "-Y" | ... }
 * Defaults: unit metre; Y-up with +Z forward (glTF 2.0 spec: assets face +Z), Z-up with -Y forward (Blender front
 * view). The output faces -Z, so a default Y-up glTF is turned by 180 deg about Y (see PI-032).
 *
 * Static meshes are baked (node transforms -> vertices, one node, one mesh, primitives merged per material).
 * Skinned / animated / morphed assets are passed through untouched under one wrapper node carrying the unit and axis
 * conversion (RETARGET etc. belong to later tasks); origin reset and merging are skipped for them.
 */
import { existsSync, readdirSync, statSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, relative, resolve, sep } from 'node:path';
import { NodeIO, type Accessor, type Document, type Material, type Node, type Primitive, type Property } from '@gltf-transform/core';
import { KHRTextureTransform, type Transform } from '@gltf-transform/extensions';
import { assertAllowed, setDefaultLedger } from './allowed.ts';
import { ExitCode, GateError } from './errors.ts';
import { loadLedger, type Ledger } from './ledger.ts';
import { assertLogicalId } from './naming.ts';
import {
  applyMat3,
  det3,
  frameRotation,
  inverseTranspose3,
  mul3,
  quatFromRotation,
  type Axis,
  type Mat3,
} from './math.ts';

export type OriginMode = 'bottom-center' | 'center' | 'none';
export type UpAxis = 'Y' | 'Z';

export interface NormalizeOptions {
  /** Work directory holding `{asset_id}/raw/` (written by ingest). */
  work: string;
  /** LICENSES.md path or an already parsed ledger. */
  ledger: string | Ledger;
  assetId: string;
  /** Output logical ID (ARCHITECTURE §11.2), e.g. `prop_crate_wood_01`. */
  logicalId: string;
  /** Model file relative to `raw/`; default: the only .gltf/.glb in there. */
  input?: string;
  /** Metres per source unit; overrides the declared unit. */
  unitScale?: number;
  up?: UpAxis;
  forward?: Axis;
  origin?: OriginMode;
  /** Bake a shared per-material KHR_texture_transform into UV0 for mesh v1 output. */
  bakeTextureTransforms?: boolean;
}

export interface NormalizeResult {
  output: string;
  input: string;
  mode: 'baked' | 'wrapped';
  unitScale: number;
  up: UpAxis;
  forward: Axis;
  triangles: number;
  bbox: { min: [number, number, number]; max: [number, number, number] };
  warnings: string[];
}

const UNITS: Record<string, number> = { m: 1, cm: 0.01, mm: 0.001, km: 1000, in: 0.0254, ft: 0.3048 };
const AXES: readonly string[] = ['+X', '-X', '+Y', '-Y', '+Z', '-Z'];

function listModels(dir: string, out: string[] = []): string[] {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) listModels(p, out);
    else if (/\.(gltf|glb)$/i.test(e.name)) out.push(p);
  }
  return out.sort();
}

function declared(doc: Document): Record<string, unknown> {
  const root = doc.getRoot();
  const a = (root.getAsset() as { extras?: Record<string, unknown> }).extras ?? {};
  return { ...root.getExtras(), ...a };
}

function parseUnit(v: unknown): number | undefined {
  if (typeof v === 'number' && v > 0) return v;
  if (typeof v === 'string' && v in UNITS) return UNITS[v];
  return undefined;
}

function parseAxis(v: unknown): Axis | undefined {
  if (typeof v !== 'string') return undefined;
  const s = v.length === 1 ? '+' + v.toUpperCase() : v.toUpperCase();
  return AXES.includes(s) ? (s as Axis) : undefined;
}

function mat4Mul(a: readonly number[], b: readonly number[]): number[] {
  const o = new Array<number>(16).fill(0);
  for (let c = 0; c < 4; c++)
    for (let r = 0; r < 4; r++) {
      let s = 0;
      for (let k = 0; k < 4; k++) s += (a[k * 4 + r] as number) * (b[c * 4 + k] as number);
      o[c * 4 + r] = s;
    }
  return o;
}

function mat3Of(m4: readonly number[]): Mat3 {
  return [m4[0], m4[1], m4[2], m4[4], m4[5], m4[6], m4[8], m4[9], m4[10]] as unknown as Mat3;
}

function readAll(acc: Accessor): Float32Array {
  const n = acc.getCount();
  const sz = acc.getElementSize();
  const out = new Float32Array(n * sz);
  const tmp = new Array<number>(sz).fill(0);
  for (let i = 0; i < n; i++) {
    acc.getElement(i, tmp);
    for (let k = 0; k < sz; k++) out[i * sz + k] = tmp[k] as number;
  }
  return out;
}

function triangleIndices(prim: Primitive, vertexCount: number, warnings: string[]): Uint32Array | null {
  const mode = prim.getMode();
  const ia = prim.getIndices();
  const idx = ia ? Uint32Array.from(ia.getArray() as ArrayLike<number>) : Uint32Array.from({ length: vertexCount }, (_, i) => i);
  if (mode === 4) return idx.subarray(0, idx.length - (idx.length % 3));
  const tris: number[] = [];
  if (mode === 5) {
    for (let i = 2; i < idx.length; i++) {
      const [a, b, c] = [idx[i - 2], idx[i - 1], idx[i]] as [number, number, number];
      if (a === b || b === c || a === c) continue; // degenerate (restart helper)
      if (i % 2 === 0) tris.push(a, b, c);
      else tris.push(b, a, c);
    }
  } else if (mode === 6) {
    for (let i = 2; i < idx.length; i++) tris.push(idx[0] as number, idx[i - 1] as number, idx[i] as number);
  } else {
    warnings.push(`dropped a primitive with non-triangle mode ${mode} (points/lines)`);
    return null;
  }
  return Uint32Array.from(tris);
}

interface Geo {
  key: string;
  material: Material | null;
  /** semantic -> interleaved float data (and component count) */
  attrs: Map<string, { size: number; data: number[] }>;
  index: number[];
  vertexCount: number;
}

function orphaned(p: Property): boolean {
  return p.listParents().every((q) => q.propertyType === 'Root');
}

/** Mesh v1 uses UV0 for all material slots. Bake a shared per-material UV transform before merging. */
function bakeTextureTransforms(doc: Document): void {
  const extension = doc.getRoot().listExtensionsUsed().find((e) => e.extensionName === 'KHR_texture_transform');
  if (!extension) return;
  for (const material of doc.getRoot().listMaterials()) {
    const infos = [material.getBaseColorTextureInfo(), material.getNormalTextureInfo(), material.getMetallicRoughnessTextureInfo(),
      material.getOcclusionTextureInfo(), material.getEmissiveTextureInfo()].filter((info) => info !== null);
    const settings = infos.map((info) => {
      const transform = info.getExtension<Transform>('KHR_texture_transform');
      return { uv: transform?.getTexCoord() ?? info.getTexCoord(), offset: transform?.getOffset() ?? [0, 0],
        scale: transform?.getScale() ?? [1, 1], rotation: transform?.getRotation() ?? 0 };
    });
    const first = settings[0];
    if (!first) continue;
    if (settings.some((value) => JSON.stringify(value) !== JSON.stringify(first))) {
      throw new GateError(ExitCode.Usage, 'KHR_texture_transform: differing UV transforms on one material are unsupported by mesh v1');
    }
    const { uv, offset: [dx, dy], scale: [sx, sy], rotation } = first;
    const cos = Math.cos(rotation), sin = Math.sin(rotation);
    for (const mesh of doc.getRoot().listMeshes()) for (const primitive of mesh.listPrimitives()) {
      if (primitive.getMaterial() !== material) continue;
      const source = primitive.getAttribute(`TEXCOORD_${uv}`);
      if (!source) throw new GateError(ExitCode.Usage, `KHR_texture_transform: missing TEXCOORD_${uv}`);
      const result = doc.createAccessor().setType('VEC2').setBuffer(source.getBuffer());
      const data = new Float32Array(source.getCount() * 2);
      const value = [0, 0];
      for (let i = 0; i < source.getCount(); i++) {
        const [u, v] = source.getElement(i, value);
        data[i * 2] = dx! + cos * sx! * u! - sin * sy! * v!;
        data[i * 2 + 1] = dy! + sin * sx! * u! + cos * sy! * v!;
      }
      primitive.setAttribute('TEXCOORD_0', result.setArray(data));
    }
    for (const info of infos) info.setTexCoord(0).setExtension('KHR_texture_transform', null);
  }
  extension.dispose();
}

export async function normalize(opts: NormalizeOptions): Promise<NormalizeResult> {
  const ledger = typeof opts.ledger === 'string' ? loadLedger(opts.ledger) : opts.ledger;
  setDefaultLedger(ledger);
  assertLogicalId(opts.logicalId);
  assertAllowed(opts.assetId, 'CONVERT', ledger);

  const assetDir = resolve(opts.work, opts.assetId);
  const rawDir = join(assetDir, 'raw');
  if (!existsSync(rawDir)) {
    throw new GateError(ExitCode.Usage, `${rawDir}: no ingested files for ${opts.assetId}; run "asset-pipeline ingest" first`, { assetId: opts.assetId });
  }
  let input: string;
  if (opts.input) {
    input = resolve(rawDir, opts.input);
    if (relative(rawDir, input).startsWith('..') || !existsSync(input) || !statSync(input).isFile()) {
      throw new GateError(ExitCode.Usage, `${opts.input}: not a file inside ${rawDir}`, { assetId: opts.assetId });
    }
  } else {
    const models = listModels(rawDir);
    if (models.length !== 1) {
      throw new GateError(ExitCode.Usage, `${rawDir}: expected exactly one .gltf/.glb, found ${models.length}; pass --input`, { assetId: opts.assetId });
    }
    input = models[0] as string;
  }

  const io = new NodeIO();
  if (opts.bakeTextureTransforms) io.registerExtensions([KHRTextureTransform]);
  const doc = await io.read(input);
  if (opts.bakeTextureTransforms) bakeTextureTransforms(doc);
  const root = doc.getRoot();
  const warnings: string[] = [];
  const decl = declared(doc);

  const unitScale = opts.unitScale ?? parseUnit(decl.unit) ?? 1;
  if (!(unitScale > 0) || !Number.isFinite(unitScale)) throw new GateError(ExitCode.Usage, `invalid unit scale ${unitScale}`);
  const declUp = typeof decl.up_axis === 'string' ? decl.up_axis.toUpperCase() : undefined;
  const up: UpAxis = opts.up ?? (declUp === 'Z' ? 'Z' : 'Y');
  const forward: Axis = opts.forward ?? parseAxis(decl.forward_axis) ?? (up === 'Z' ? '-Y' : '+Z');
  const origin: OriginMode = opts.origin ?? 'bottom-center';
  let rot: Mat3;
  try {
    rot = frameRotation(`+${up}` as Axis, forward);
  } catch (e) {
    throw new GateError(ExitCode.Usage, (e as Error).message, { assetId: opts.assetId });
  }
  const conv: Mat3 = mul3([unitScale, 0, 0, 0, unitScale, 0, 0, 0, unitScale] as unknown as Mat3, rot);
  const conv4 = [conv[0], conv[1], conv[2], 0, conv[3], conv[4], conv[5], 0, conv[6], conv[7], conv[8], 0, 0, 0, 0, 1];

  const scene = root.getDefaultScene() ?? root.listScenes()[0];
  if (!scene) throw new GateError(ExitCode.Usage, `${input}: glTF has no scene`, { assetId: opts.assetId });
  if (root.listScenes().length > 1) warnings.push(`${root.listScenes().length} scenes: only the default scene is kept`);

  const dynamic = root.listSkins().length > 0 || root.listAnimations().length > 0 || root.listMeshes().some((m) => m.listPrimitives().some((p) => p.listTargets().length > 0));
  let triangles = 0;
  let bboxMin: [number, number, number] = [Infinity, Infinity, Infinity];
  let bboxMax: [number, number, number] = [-Infinity, -Infinity, -Infinity];
  const grow = (x: number, y: number, z: number): void => {
    bboxMin = [Math.min(bboxMin[0], x), Math.min(bboxMin[1], y), Math.min(bboxMin[2], z)];
    bboxMax = [Math.max(bboxMax[0], x), Math.max(bboxMax[1], y), Math.max(bboxMax[2], z)];
  };

  let mode: 'baked' | 'wrapped';
  if (dynamic) {
    mode = 'wrapped';
    warnings.push('skinned/animated/morphed asset: passed through under a unit+axis wrapper node; origin reset and mesh merging skipped (RETARGET etc. are later tasks)');
    const wrapper = doc.createNode(opts.logicalId).setScale([unitScale, unitScale, unitScale]).setRotation(quatFromRotation(rot));
    for (const child of scene.listChildren()) {
      scene.removeChild(child);
      wrapper.addChild(child);
    }
    scene.addChild(wrapper);
    // Report triangles / bbox of the bind pose for information.
    scene.traverse((node: Node) => {
      const mesh = node.getMesh();
      if (!mesh) return;
      const world = mat4Mul(conv4, node.getWorldMatrix());
      const m3 = mat3Of(world);
      for (const prim of mesh.listPrimitives()) {
        const pos = prim.getAttribute('POSITION');
        if (!pos) continue;
        const p = readAll(pos);
        for (let i = 0; i < p.length; i += 3) {
          const v = applyMat3(m3, p[i] as number, p[i + 1] as number, p[i + 2] as number);
          grow(v[0] + (world[12] as number), v[1] + (world[13] as number), v[2] + (world[14] as number));
        }
        const ia = prim.getIndices();
        triangles += Math.floor((ia ? ia.getCount() : pos.getCount()) / 3);
      }
    });
  } else {
    mode = 'baked';
    const groups = new Map<string, Geo>();
    scene.traverse((node: Node) => {
      const mesh = node.getMesh();
      if (!mesh) return;
      const C = mat4Mul(conv4, node.getWorldMatrix());
      const C3 = mat3Of(C);
      const flip = det3(C3) < 0;
      const NM = inverseTranspose3(C3);
      for (const prim of mesh.listPrimitives()) {
        const posAcc = prim.getAttribute('POSITION');
        if (!posAcc) continue;
        const vc = posAcc.getCount();
        const idx = triangleIndices(prim, vc, warnings);
        if (!idx || idx.length === 0) continue;
        const sems = prim
          .listSemantics()
          .filter((s) => s === 'POSITION' || s === 'NORMAL' || s === 'TANGENT' || /^TEXCOORD_\d+$/.test(s) || /^COLOR_\d+$/.test(s))
          .sort();
        for (const s of prim.listSemantics()) if (!sems.includes(s)) warnings.push(`attribute ${s} dropped`);
        const material = prim.getMaterial();
        const key = `${material ? doc.getRoot().listMaterials().indexOf(material) : -1}|${sems.map((s) => `${s}:${prim.getAttribute(s)?.getElementSize()}`).join(',')}`;
        let g = groups.get(key);
        if (!g) {
          g = { key, material, attrs: new Map(sems.map((s) => [s, { size: (prim.getAttribute(s) as Accessor).getElementSize(), data: [] }])), index: [], vertexCount: 0 };
          groups.set(key, g);
        }
        const base = g.vertexCount;
        for (const s of sems) {
          const acc = prim.getAttribute(s) as Accessor;
          const src = readAll(acc);
          const dst = (g.attrs.get(s) as { data: number[] }).data;
          const sz = acc.getElementSize();
          for (let i = 0; i < acc.getCount(); i++) {
            const a = src[i * sz] as number, b = src[i * sz + 1] as number, c = src[i * sz + 2] as number;
            if (s === 'POSITION') {
              const v = applyMat3(C3, a, b, c);
              dst.push(v[0] + (C[12] as number), v[1] + (C[13] as number), v[2] + (C[14] as number));
            } else if (s === 'NORMAL') {
              const v = applyMat3(NM, a, b, c);
              const l = Math.hypot(v[0], v[1], v[2]) || 1;
              dst.push(v[0] / l, v[1] / l, v[2] / l);
            } else if (s === 'TANGENT') {
              const v = applyMat3(C3, a, b, c);
              const l = Math.hypot(v[0], v[1], v[2]) || 1;
              dst.push(v[0] / l, v[1] / l, v[2] / l, flip ? -(src[i * sz + 3] as number) : (src[i * sz + 3] as number));
            } else {
              for (let k = 0; k < sz; k++) dst.push(src[i * sz + k] as number);
            }
          }
        }
        for (let i = 0; i < idx.length; i += 3) {
          const [a, b, c] = [idx[i], idx[i + 1], idx[i + 2]] as [number, number, number];
          g.index.push(base + a, base + (flip ? c : b), base + (flip ? b : c));
        }
        g.vertexCount += vc;
        triangles += idx.length / 3;
      }
    });

    // Origin reset.
    for (const g of groups.values()) {
      const p = (g.attrs.get('POSITION') as { data: number[] }).data;
      for (let i = 0; i < p.length; i += 3) grow(p[i] as number, p[i + 1] as number, p[i + 2] as number);
    }
    if (triangles === 0) throw new GateError(ExitCode.Usage, `${input}: no triangle geometry found`, { assetId: opts.assetId });
    let shift: [number, number, number] = [0, 0, 0];
    if (origin === 'bottom-center') shift = [-(bboxMin[0] + bboxMax[0]) / 2, -bboxMin[1], -(bboxMin[2] + bboxMax[2]) / 2];
    else if (origin === 'center') shift = [-(bboxMin[0] + bboxMax[0]) / 2, -(bboxMin[1] + bboxMax[1]) / 2, -(bboxMin[2] + bboxMax[2]) / 2];
    for (const g of groups.values()) {
      const p = (g.attrs.get('POSITION') as { data: number[] }).data;
      for (let i = 0; i < p.length; i += 3) {
        p[i] = (p[i] as number) + shift[0];
        p[i + 1] = (p[i + 1] as number) + shift[1];
        p[i + 2] = (p[i + 2] as number) + shift[2];
      }
    }
    bboxMin = [bboxMin[0] + shift[0], bboxMin[1] + shift[1], bboxMin[2] + shift[2]];
    bboxMax = [bboxMax[0] + shift[0], bboxMax[1] + shift[1], bboxMax[2] + shift[2]];

    // Rebuild the scene: one node, one mesh, one primitive per material/attribute-set group.
    for (const n of root.listNodes()) n.dispose();
    for (const m of root.listMeshes()) {
      for (const p of m.listPrimitives()) p.dispose(); // detach the old accessors so they become orphans
      m.dispose();
    }
    for (const s of root.listScenes()) if (s !== scene) s.dispose();
    for (const a of root.listAccessors()) if (orphaned(a)) a.dispose();
    const inUse = new Set([...groups.values()].map((g) => g.material));
    for (const m of root.listMaterials()) if (!inUse.has(m) && orphaned(m)) m.dispose();
    for (const t of root.listTextures()) if (orphaned(t)) t.dispose();
    for (const b of root.listBuffers()) b.dispose();
    const buffer = doc.createBuffer('buffer').setURI('normalized.bin');
    const outMesh = doc.createMesh(opts.logicalId);
    for (const g of [...groups.values()].sort((a, b) => (a.key < b.key ? -1 : 1))) {
      const prim = doc.createPrimitive();
      for (const [sem, { size, data }] of g.attrs) {
        const type = size === 4 ? 'VEC4' : size === 3 ? 'VEC3' : size === 2 ? 'VEC2' : 'SCALAR';
        prim.setAttribute(sem, doc.createAccessor(sem.toLowerCase()).setType(type).setArray(new Float32Array(data)).setBuffer(buffer));
      }
      const idxArr = g.vertexCount <= 65535 ? new Uint16Array(g.index) : new Uint32Array(g.index);
      prim.setIndices(doc.createAccessor('indices').setType('SCALAR').setArray(idxArr).setBuffer(buffer));
      if (g.material) prim.setMaterial(g.material);
      outMesh.addPrimitive(prim);
    }
    scene.addChild(doc.createNode(opts.logicalId).setMesh(outMesh));
  }

  const asset = root.getAsset() as { extras?: Record<string, unknown>; generator?: string };
  asset.generator = 'frontline asset-pipeline normalize (M1-001)';
  asset.extras = {
    frontline: { unit: 'm', up: 'Y', forward: '-Z', source: { unit_scale: unitScale, up, forward, origin: mode === 'baked' ? origin : 'none', mode } },
  };
  root.setExtras({});

  mkdirSync(assetDir, { recursive: true });
  const output = join(assetDir, 'normalized.gltf');
  await io.write(output, doc);
  const result: NormalizeResult = {
    output,
    input,
    mode,
    unitScale,
    up,
    forward,
    triangles,
    bbox: { min: bboxMin, max: bboxMax },
    warnings,
  };
  writeFileSync(
    join(assetDir, 'normalized.json'),
    JSON.stringify({ asset_id: opts.assetId, logical_id: opts.logicalId, ...result, input: relative(assetDir, input).split(sep).join('/'), output: 'normalized.gltf' }, null, 2) + '\n',
  );
  return result;
}
