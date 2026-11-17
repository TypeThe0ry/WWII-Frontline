/**
 * Decoders for the `.fpak` entry layouts (SPEC.md): mesh layout v1 (meshopt vertex/index buffers) and texture layout v1
 * (KTX2 payload pieces). Pure functions over bytes; the meshopt implementation is injected so the same code runs in a Worker,
 * on the main thread and in vitest. No Node APIs.
 */
export interface MeshoptLike {
  ready?: Promise<void>;
  decodeVertexBuffer(target: Uint8Array, count: number, size: number, source: Uint8Array): void;
  decodeIndexBuffer(target: Uint8Array, count: number, size: number, source: Uint8Array): void;
}

const SEMANTICS = ['POSITION', 'NORMAL', 'TANGENT', 'TEXCOORD_0', 'TEXCOORD_1', 'COLOR_0', 'JOINTS_0', 'WEIGHTS_0'] as const;
const COMPONENT_TYPES = [5120, 5121, 5122, 5123, 5125, 5126] as const;
const COMPONENT_BYTES = [1, 1, 2, 2, 4, 4] as const;
export const NO_MATERIAL = 0xffff;

export interface DecodedAttribute {
  semantic: string;
  componentType: number;
  elements: number;
  normalized: boolean;
  data: Uint8Array;
}
export interface DecodedPrimitive {
  materialIndex: number;
  matrix: number[];
  vertexCount: number;
  indices: Uint32Array;
  attributes: DecodedAttribute[];
}
export interface DecodedMaterial {
  baseColor: [number, number, number, number];
  metallic: number;
  roughness: number;
  emissive: [number, number, number];
  /** texture entry logical ids: base_color, normal, orm, emissive ('' = none) */
  textures: [string, string, string, string];
}
export interface DecodedMesh {
  bbox: { min: [number, number, number]; max: [number, number, number] };
  materials: DecodedMaterial[];
  primitives: DecodedPrimitive[];
}

export class EntryFormatError extends Error {}

const align4 = (n: number) => Math.ceil(n / 4) * 4;

export async function decodeMesh(b: Uint8Array, meshopt: MeshoptLike): Promise<DecodedMesh> {
  await meshopt.ready;
  const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
  const dec = new TextDecoder();
  let p = 0;
  const u8 = () => dv.getUint8(p++);
  const u16 = () => ((p += 2), dv.getUint16(p - 2, true));
  const u32 = () => ((p += 4), dv.getUint32(p - 4, true));
  const f32 = () => ((p += 4), dv.getFloat32(p - 4, true));
  if (u8() !== 1) throw new EntryFormatError('unknown mesh layout version');
  u8();
  const nPrim = u16();
  const nMat = u16();
  u16();
  const bbox = { min: [f32(), f32(), f32()] as [number, number, number], max: [f32(), f32(), f32()] as [number, number, number] };
  const materials: DecodedMaterial[] = [];
  for (let i = 0; i < nMat; i++) {
    const baseColor: [number, number, number, number] = [f32(), f32(), f32(), f32()];
    const metallic = f32();
    const roughness = f32();
    const emissive: [number, number, number] = [f32(), f32(), f32()];
    const textures: [string, string, string, string] = ['', '', '', ''];
    for (let s = 0; s < 4; s++) {
      const n = u8();
      textures[s] = dec.decode(b.subarray(p, p + n));
      p += n;
    }
    p = align4(p);
    materials.push({ baseColor, metallic, roughness, emissive, textures });
  }
  const primitives: DecodedPrimitive[] = [];
  for (let i = 0; i < nPrim; i++) {
    const materialIndex = u16();
    const nAttr = u8();
    const indexSize = u8();
    if (indexSize !== 2 && indexSize !== 4) throw new EntryFormatError(`bad index size ${indexSize}`);
    const vertexCount = u32();
    const indexCount = u32();
    const vbBytes = u32();
    const ibBytes = u32();
    const matrix = Array.from({ length: 16 }, () => f32());
    const defs = Array.from({ length: nAttr }, () => {
      const semantic = SEMANTICS[u8()];
      const ct = COMPONENT_TYPES[u8()];
      const elements = u8();
      const normalized = u8() === 1;
      if (semantic === undefined || ct === undefined) throw new EntryFormatError('bad attribute code');
      return { semantic: semantic as string, componentType: ct as number, elements, normalized };
    });
    const compBytes = (ct: number) => COMPONENT_BYTES[COMPONENT_TYPES.indexOf(ct as (typeof COMPONENT_TYPES)[number])] as number;
    const sizes = defs.map((d) => align4(d.elements * compBytes(d.componentType)));
    const stride = sizes.reduce((s, x) => s + x, 0);
    const inter = new Uint8Array(stride * vertexCount);
    meshopt.decodeVertexBuffer(inter, vertexCount, stride, b.subarray(p, p + vbBytes));
    p = align4(p + vbBytes);
    const idxRaw = new Uint8Array(indexCount * indexSize);
    meshopt.decodeIndexBuffer(idxRaw, indexCount, indexSize, b.subarray(p, p + ibBytes));
    p = align4(p + ibBytes);
    const indices = indexSize === 2 ? Uint32Array.from(new Uint16Array(idxRaw.buffer, 0, indexCount)) : new Uint32Array(idxRaw.buffer.slice(0, indexCount * 4));
    let off = 0;
    const attributes = defs.map((d, k) => {
      const ab = d.elements * compBytes(d.componentType);
      const data = new Uint8Array(ab * vertexCount);
      for (let v = 0; v < vertexCount; v++) data.set(inter.subarray(v * stride + off, v * stride + off + ab), v * ab);
      off += sizes[k] as number;
      return { ...d, data };
    });
    primitives.push({ materialIndex, matrix, vertexCount, indices, attributes });
  }
  return { bbox, materials, primitives };
}

export interface DecodedTexture {
  vkFormat: number;
  width: number;
  height: number;
  typeSize: number;
  depth: number;
  layerCount: number;
  faceCount: number;
  supercompressionScheme: number;
  channels: number;
  /** `fl_layout` key/value from the encoder: '' | 'RRRG' (normal maps keep X in R and Y in G). */
  layout: string;
  dfd: Uint8Array;
  sgd: Uint8Array;
  /** Smallest mip first, as stored. */
  levels: { data: Uint8Array; uncompressedByteLength: number }[];
}

export function decodeTexture(b: Uint8Array): DecodedTexture {
  const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
  if (dv.getUint8(0) !== 1) throw new EntryFormatError('unknown texture layout version');
  const nLevels = dv.getUint8(2);
  const dfdAt = dv.getUint32(32, true);
  const dfdLen = dv.getUint32(36, true);
  const sgdAt = dv.getUint32(40, true);
  const sgdLen = dv.getUint32(44, true);
  const layoutLen = dv.getUint16(48, true);
  const layout = new TextDecoder().decode(b.subarray(50, 50 + layoutLen));
  const head = 48 + 2 + layoutLen;
  const dataAt = Math.ceil((sgdAt + sgdLen) / 16) * 16;
  const levels: DecodedTexture['levels'] = [];
  for (let i = 0; i < nLevels; i++) {
    const at = dv.getUint32(head + i * 12, true);
    const len = dv.getUint32(head + i * 12 + 4, true);
    levels.push({ data: b.slice(dataAt + at, dataAt + at + len), uncompressedByteLength: dv.getUint32(head + i * 12 + 8, true) });
  }
  return {
    vkFormat: dv.getUint32(4, true), width: dv.getUint32(12, true), height: dv.getUint32(16, true),
    typeSize: dv.getUint32(8, true), depth: dv.getUint32(20, true), layerCount: dv.getUint32(24, true), faceCount: dv.getUint32(28, true), supercompressionScheme: dv.getUint8(1),
    channels: dv.getUint8(3), layout, dfd: b.slice(dfdAt, dfdAt + dfdLen), sgd: b.slice(sgdAt, sgdAt + sgdLen), levels,
  };
}
