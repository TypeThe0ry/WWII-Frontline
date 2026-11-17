/**
 * Mesh entry layout v1 (codec `Codec.meshV1`): own small header + meshopt-compressed vertex / index buffers per primitive.
 * This is deliberately not glTF/GLB: there is no JSON chunk, no accessors and no standard magic, so generic viewers
 * cannot open it. It is NOT a protection mechanism (see format.ts).
 *
 * Little-endian. Offsets in SPEC.md ("Mesh entry").
 */
import type { Document, Primitive, Texture } from '@gltf-transform/core';
import { MeshoptDecoder, MeshoptEncoder } from 'meshoptimizer';
import { FpakFormatError } from './format.ts';

export const MESH_LAYOUT_VERSION = 1;
export const NO_MATERIAL = 0xffff;

const SEMANTICS = ['POSITION', 'NORMAL', 'TANGENT', 'TEXCOORD_0', 'TEXCOORD_1', 'COLOR_0', 'JOINTS_0', 'WEIGHTS_0'] as const;
const COMPONENT_TYPES = [5120, 5121, 5122, 5123, 5125, 5126] as const;
const COMPONENT_BYTES = [1, 1, 2, 2, 4, 4] as const;
const TEXTURE_SLOTS = ['base_color', 'normal', 'orm', 'emissive'] as const;

export interface MeshAttribute {
  semantic: string;
  componentType: number;
  elements: number;
  normalized: boolean;
  /** Tightly packed little-endian element data (count * elements * component bytes). */
  data: Uint8Array;
}
export interface MeshMaterial {
  baseColor: [number, number, number, number];
  metallic: number;
  roughness: number;
  emissive: [number, number, number];
  /** Logical ids of texture entries (empty string = none), in TEXTURE_SLOTS order. */
  textures: [string, string, string, string];
}
export interface MeshPrimitive {
  materialIndex: number;
  /** Column-major 4x4 applied to POSITION (carries the dequantization / node transform). */
  matrix: number[];
  vertexCount: number;
  indexSize: 2 | 4;
  indices: Uint32Array;
  attributes: MeshAttribute[];
}
export interface MeshEntry {
  bbox: { min: [number, number, number]; max: [number, number, number] };
  materials: MeshMaterial[];
  primitives: MeshPrimitive[];
}

export interface EncodeMeshOptions {
  /** Logical id of the texture entry a glTF texture was packed as. */
  textureId: (t: Texture) => string;
}

class Out {
  #parts: Uint8Array[] = [];
  #len = 0;
  u8(v: number) {
    this.raw(Uint8Array.of(v));
  }
  u16(v: number) {
    const b = new Uint8Array(2);
    new DataView(b.buffer).setUint16(0, v, true);
    this.raw(b);
  }
  u32(v: number) {
    const b = new Uint8Array(4);
    new DataView(b.buffer).setUint32(0, v, true);
    this.raw(b);
  }
  f32(...v: number[]) {
    const b = new Uint8Array(4 * v.length);
    const dv = new DataView(b.buffer);
    v.forEach((x, i) => dv.setFloat32(i * 4, x, true));
    this.raw(b);
  }
  raw(b: Uint8Array) {
    this.#parts.push(b);
    this.#len += b.byteLength;
  }
  pad4() {
    const p = (4 - (this.#len % 4)) % 4;
    if (p) this.raw(new Uint8Array(p));
  }
  done(): Uint8Array {
    const o = new Uint8Array(this.#len);
    let p = 0;
    for (const b of this.#parts) {
      o.set(b, p);
      p += b.byteLength;
    }
    return o;
  }
}

function typeCode(componentType: number): number {
  const i = (COMPONENT_TYPES as readonly number[]).indexOf(componentType);
  if (i < 0) throw new FpakFormatError(`unsupported component type ${componentType}`);
  return i;
}

function attrBytes(a: MeshAttribute): number {
  return a.elements * (COMPONENT_BYTES[typeCode(a.componentType)] as number);
}

const GL_BY_ARRAY: [string, number][] = [['Int8Array', 5120], ['Uint8Array', 5121], ['Int16Array', 5122], ['Uint16Array', 5123], ['Uint32Array', 5125], ['Float32Array', 5126]];

function primitiveToMesh(p: Primitive, matrix: number[], materialIndex: number): MeshPrimitive {
  const attributes: MeshAttribute[] = [];
  for (const sem of SEMANTICS) {
    const acc = p.getAttribute(sem);
    if (!acc) continue;
    const arr = acc.getArray();
    if (!arr) continue;
    const ct = GL_BY_ARRAY.find(([n]) => n === arr.constructor.name)?.[1];
    if (ct === undefined) throw new FpakFormatError(`${sem}: unsupported array type ${arr.constructor.name}`);
    attributes.push({
      semantic: sem, componentType: ct, elements: acc.getElementSize(), normalized: acc.getNormalized(),
      data: new Uint8Array(arr.buffer.slice(arr.byteOffset, arr.byteOffset + arr.byteLength)),
    });
  }
  const pos = p.getAttribute('POSITION');
  if (!pos) throw new FpakFormatError('primitive without POSITION');
  const idx = p.getIndices();
  const vertexCount = pos.getCount();
  const indices = idx ? Uint32Array.from(idx.getArray() as ArrayLike<number>) : Uint32Array.from({ length: vertexCount }, (_, i) => i);
  return { materialIndex, matrix, vertexCount, indexSize: vertexCount <= 0xffff ? 2 : 4, indices, attributes };
}

/** Collect a static glTF document (as written by `process`, one or more mesh nodes) into the neutral MeshEntry. */
export function meshEntryFromDocument(doc: Document, o: EncodeMeshOptions): MeshEntry {
  const root = doc.getRoot();
  if (root.listSkins().length || root.listAnimations().length) {
    throw new FpakFormatError('skinned / animated meshes are not supported by mesh entry v1 yet (character assets: M1-045)');
  }
  const mats = root.listMaterials();
  const materials: MeshMaterial[] = mats.map((m) => {
    const orm = m.getOcclusionTexture() ?? m.getMetallicRoughnessTexture();
    const id = (t: Texture | null) => (t ? o.textureId(t) : '');
    return {
      baseColor: m.getBaseColorFactor() as [number, number, number, number],
      metallic: m.getMetallicFactor(),
      roughness: m.getRoughnessFactor(),
      emissive: m.getEmissiveFactor() as [number, number, number],
      textures: [id(m.getBaseColorTexture()), id(m.getNormalTexture()), id(orm), id(m.getEmissiveTexture())],
    };
  });
  const primitives: MeshPrimitive[] = [];
  for (const node of root.listNodes()) {
    const mesh = node.getMesh();
    if (!mesh) continue;
    const matrix = Array.from(node.getWorldMatrix());
    for (const p of mesh.listPrimitives()) {
      if (p.getMode() !== 4) continue;
      const m = p.getMaterial();
      primitives.push(primitiveToMesh(p, matrix, m ? mats.indexOf(m) : NO_MATERIAL));
    }
  }
  if (primitives.length === 0) throw new FpakFormatError('document has no triangle primitives');
  return { bbox: bboxOf(doc, primitives), materials, primitives };
}

function bboxOf(doc: Document, prims: MeshPrimitive[]): MeshEntry['bbox'] {
  void doc;
  const min: [number, number, number] = [Infinity, Infinity, Infinity];
  const max: [number, number, number] = [-Infinity, -Infinity, -Infinity];
  for (const p of prims) {
    const a = p.attributes.find((x) => x.semantic === 'POSITION') as MeshAttribute;
    const dv = new DataView(a.data.buffer, a.data.byteOffset, a.data.byteLength);
    const bytes = COMPONENT_BYTES[typeCode(a.componentType)] as number;
    const read = (i: number): number => {
      const o = i * bytes;
      switch (a.componentType) {
        case 5126: return dv.getFloat32(o, true);
        case 5122: return a.normalized ? Math.max(dv.getInt16(o, true) / 32767, -1) : dv.getInt16(o, true);
        case 5123: return a.normalized ? dv.getUint16(o, true) / 65535 : dv.getUint16(o, true);
        case 5120: return a.normalized ? Math.max(dv.getInt8(o) / 127, -1) : dv.getInt8(o);
        default: return a.normalized ? dv.getUint8(o) / 255 : dv.getUint8(o);
      }
    };
    const m = p.matrix;
    for (let v = 0; v < p.vertexCount; v++) {
      const x = read(v * 3);
      const y = read(v * 3 + 1);
      const z = read(v * 3 + 2);
      const w = [
        (m[0] as number) * x + (m[4] as number) * y + (m[8] as number) * z + (m[12] as number),
        (m[1] as number) * x + (m[5] as number) * y + (m[9] as number) * z + (m[13] as number),
        (m[2] as number) * x + (m[6] as number) * y + (m[10] as number) * z + (m[14] as number),
      ];
      for (let k = 0; k < 3; k++) {
        min[k] = Math.min(min[k] as number, w[k] as number);
        max[k] = Math.max(max[k] as number, w[k] as number);
      }
    }
  }
  return { min, max };
}

function interleave(attrs: MeshAttribute[], count: number): { buf: Uint8Array; stride: number } {
  const sizes = attrs.map((a) => Math.ceil(attrBytes(a) / 4) * 4);
  const stride = sizes.reduce((s, x) => s + x, 0);
  const buf = new Uint8Array(stride * count);
  let off = 0;
  attrs.forEach((a, k) => {
    const ab = attrBytes(a);
    for (let v = 0; v < count; v++) buf.set(a.data.subarray(v * ab, (v + 1) * ab), v * stride + off);
    off += sizes[k] as number;
  });
  return { buf, stride };
}

export async function encodeMeshEntry(e: MeshEntry): Promise<Uint8Array> {
  await MeshoptEncoder.ready;
  const o = new Out();
  o.u8(MESH_LAYOUT_VERSION);
  o.u8(0); // flags
  o.u16(e.primitives.length);
  o.u16(e.materials.length);
  o.u16(0);
  o.f32(...e.bbox.min, ...e.bbox.max);
  for (const m of e.materials) {
    o.f32(...m.baseColor, m.metallic, m.roughness, ...m.emissive);
    for (const t of m.textures) {
      const b = Buffer.from(t);
      o.u8(b.length);
      o.raw(b);
    }
    o.pad4();
  }
  for (const p of e.primitives) {
    const { buf, stride } = interleave(p.attributes, p.vertexCount);
    if (stride > 256) throw new FpakFormatError(`vertex stride ${stride} exceeds 256`);
    const vb = MeshoptEncoder.encodeVertexBuffer(buf, p.vertexCount, stride);
    const raw = p.indexSize === 2 ? new Uint8Array(Uint16Array.from(p.indices).buffer) : new Uint8Array(p.indices.buffer, p.indices.byteOffset, p.indices.byteLength);
    const ib = MeshoptEncoder.encodeIndexBuffer(raw, p.indices.length, p.indexSize);
    o.u16(p.materialIndex);
    o.u8(p.attributes.length);
    o.u8(p.indexSize);
    o.u32(p.vertexCount);
    o.u32(p.indices.length);
    o.u32(vb.byteLength);
    o.u32(ib.byteLength);
    o.f32(...p.matrix);
    for (const a of p.attributes) {
      o.u8(SEMANTICS.indexOf(a.semantic as (typeof SEMANTICS)[number]));
      o.u8(typeCode(a.componentType));
      o.u8(a.elements);
      o.u8(a.normalized ? 1 : 0);
    }
    o.raw(vb);
    o.pad4();
    o.raw(ib);
    o.pad4();
  }
  return o.done();
}

export async function decodeMeshEntry(b: Uint8Array): Promise<MeshEntry> {
  await MeshoptDecoder.ready;
  const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
  let p = 0;
  const u8 = () => dv.getUint8(p++);
  const u16 = () => ((p += 2), dv.getUint16(p - 2, true));
  const u32 = () => ((p += 4), dv.getUint32(p - 4, true));
  const f32 = () => ((p += 4), dv.getFloat32(p - 4, true));
  if (u8() !== MESH_LAYOUT_VERSION) throw new FpakFormatError('unknown mesh layout version');
  u8();
  const nPrim = u16();
  const nMat = u16();
  u16();
  const bbox = { min: [f32(), f32(), f32()] as [number, number, number], max: [f32(), f32(), f32()] as [number, number, number] };
  const materials: MeshMaterial[] = [];
  for (let i = 0; i < nMat; i++) {
    const baseColor: [number, number, number, number] = [f32(), f32(), f32(), f32()];
    const metallic = f32();
    const roughness = f32();
    const emissive: [number, number, number] = [f32(), f32(), f32()];
    const textures = ['', '', '', ''] as MeshMaterial['textures'];
    for (let s = 0; s < TEXTURE_SLOTS.length; s++) {
      const n = u8();
      textures[s] = new TextDecoder().decode(b.subarray(p, p + n));
      p += n;
    }
    p = Math.ceil(p / 4) * 4;
    materials.push({ baseColor, metallic, roughness, emissive, textures });
  }
  const primitives: MeshPrimitive[] = [];
  for (let i = 0; i < nPrim; i++) {
    const materialIndex = u16();
    const nAttr = u8();
    const indexSize = u8() as 2 | 4;
    const vertexCount = u32();
    const indexCount = u32();
    const vbBytes = u32();
    const ibBytes = u32();
    const matrix = Array.from({ length: 16 }, () => f32());
    const defs = Array.from({ length: nAttr }, () => {
      const sem = SEMANTICS[u8()] as string;
      const ct = COMPONENT_TYPES[u8()] as number;
      const elements = u8();
      return { semantic: sem, componentType: ct, elements, normalized: u8() === 1 };
    });
    const sizes = defs.map((d) => Math.ceil((d.elements * (COMPONENT_BYTES[typeCode(d.componentType)] as number)) / 4) * 4);
    const stride = sizes.reduce((s, x) => s + x, 0);
    const inter = new Uint8Array(stride * vertexCount);
    MeshoptDecoder.decodeVertexBuffer(inter, vertexCount, stride, b.subarray(p, p + vbBytes));
    p = Math.ceil((p + vbBytes) / 4) * 4;
    const idxRaw = new Uint8Array(indexCount * indexSize);
    MeshoptDecoder.decodeIndexBuffer(idxRaw, indexCount, indexSize, b.subarray(p, p + ibBytes));
    p = Math.ceil((p + ibBytes) / 4) * 4;
    const indices = indexSize === 2 ? Uint32Array.from(new Uint16Array(idxRaw.buffer, 0, indexCount)) : new Uint32Array(idxRaw.buffer.slice(0, indexCount * 4));
    let off = 0;
    const attributes = defs.map((d, k) => {
      const ab = d.elements * (COMPONENT_BYTES[typeCode(d.componentType)] as number);
      const data = new Uint8Array(ab * vertexCount);
      for (let v = 0; v < vertexCount; v++) data.set(inter.subarray(v * stride + off, v * stride + off + ab), v * ab);
      off += sizes[k] as number;
      return { ...d, data };
    });
    primitives.push({ materialIndex, matrix, vertexCount, indexSize, indices, attributes });
  }
  return { bbox, materials, primitives };
}
