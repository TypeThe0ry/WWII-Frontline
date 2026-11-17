/** Self-made synthetic geometry and textures for the M1-002 tests (no third-party content). */
import { Document, NodeIO, type Material } from '@gltf-transform/core';
import { PNG } from 'pngjs';

export interface Raster {
  width: number;
  height: number;
  data: Uint8Array; // RGBA
}

export function png(width: number, height: number, f: (x: number, y: number) => [number, number, number, number]): Buffer {
  const p = new PNG({ width, height });
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const [r, g, b, a] = f(x, y);
      const i = (y * width + x) * 4;
      p.data[i] = r;
      p.data[i + 1] = g;
      p.data[i + 2] = b;
      p.data[i + 3] = a;
    }
  }
  return PNG.sync.write(p);
}

/** Deterministic integer hash noise in [0,1). */
export function hash2(x: number, y: number, seed = 0): number {
  let h = (Math.imul(x | 0, 374761393) + Math.imul(y | 0, 668265263) + Math.imul(seed | 0, 2147483647)) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

/** Procedural albedo: gradient + mild value noise (a plausible painted/texture-like image, not incompressible noise). */
export function albedoPng(size: number, seed = 1): Buffer {
  return png(size, size, (x, y) => {
    const n = 0.85 + 0.15 * hash2(x >> 2, y >> 2, seed);
    const u = x / size;
    const v = y / size;
    return [Math.round(255 * (0.3 + 0.5 * u) * n), Math.round(255 * (0.4 + 0.4 * v) * n), Math.round(255 * 0.35 * n), 255];
  });
}

export function normalPng(size: number): Buffer {
  return png(size, size, (x, y) => {
    const nx = 0.35 * Math.sin((x / size) * 40);
    const ny = 0.35 * Math.cos((y / size) * 31);
    const nz = Math.sqrt(1 - nx * nx - ny * ny);
    return [Math.round((nx * 0.5 + 0.5) * 255), Math.round((ny * 0.5 + 0.5) * 255), Math.round((nz * 0.5 + 0.5) * 255), 255];
  });
}

export function grayPng(size: number, f: (u: number, v: number) => number): Buffer {
  return png(size, size, (x, y) => {
    const g = Math.max(0, Math.min(255, Math.round(255 * f(x / size, y / size))));
    return [g, g, g, 255];
  });
}

interface MeshData {
  pos: number[];
  nor: number[];
  uv: number[];
  idx: number[];
}

/** UV sphere with `lon` x `lat` quads (2*lon*lat triangles, minus the pole degenerates are kept as triangles). A UV seam runs along one meridian. */
export function sphereData(lon: number, lat: number, radius = 0.5): MeshData {
  const pos: number[] = [];
  const nor: number[] = [];
  const uv: number[] = [];
  const idx: number[] = [];
  for (let j = 0; j <= lat; j++) {
    const th = (j / lat) * Math.PI;
    for (let i = 0; i <= lon; i++) {
      const ph = (i / lon) * 2 * Math.PI;
      const x = Math.sin(th) * Math.cos(ph);
      const y = Math.cos(th);
      const z = Math.sin(th) * Math.sin(ph);
      pos.push(x * radius, y * radius, z * radius);
      nor.push(x, y, z);
      uv.push(i / lon, j / lat);
    }
  }
  const row = lon + 1;
  for (let j = 0; j < lat; j++) {
    for (let i = 0; i < lon; i++) {
      const a = j * row + i;
      const b = a + 1;
      const c = a + row;
      const d = c + 1;
      if (j > 0) idx.push(a, c, b);
      if (j < lat - 1) idx.push(b, c, d);
    }
  }
  return { pos, nor, uv, idx };
}

/** Cube with an `n` x `n` grid per face and one UV island per face (3x2 atlas) -> all cube edges are UV seams. */
export function seamCubeData(n: number, size = 1): MeshData {
  const pos: number[] = [];
  const nor: number[] = [];
  const uv: number[] = [];
  const idx: number[] = [];
  const h = size / 2;
  const faces: { o: [number, number, number]; u: [number, number, number]; v: [number, number, number]; n: [number, number, number] }[] = [
    { o: [h, -h, h], u: [0, 0, -size], v: [0, size, 0], n: [1, 0, 0] },
    { o: [-h, -h, -h], u: [0, 0, size], v: [0, size, 0], n: [-1, 0, 0] },
    { o: [-h, h, h], u: [size, 0, 0], v: [0, 0, -size], n: [0, 1, 0] },
    { o: [-h, -h, -h], u: [size, 0, 0], v: [0, 0, size], n: [0, -1, 0] },
    { o: [-h, -h, h], u: [size, 0, 0], v: [0, size, 0], n: [0, 0, 1] },
    { o: [h, -h, -h], u: [-size, 0, 0], v: [0, size, 0], n: [0, 0, -1] },
  ];
  faces.forEach((f, fi) => {
    const base = pos.length / 3;
    const cu = fi % 3;
    const cv = Math.floor(fi / 3);
    for (let j = 0; j <= n; j++) {
      for (let i = 0; i <= n; i++) {
        const s = i / n;
        const t = j / n;
        for (let k = 0; k < 3; k++) pos.push((f.o[k] as number) + (f.u[k] as number) * s + (f.v[k] as number) * t);
        nor.push(...f.n);
        uv.push((cu + s * 0.96 + 0.02) / 3, (cv + t * 0.96 + 0.02) / 2);
      }
    }
    for (let j = 0; j < n; j++) {
      for (let i = 0; i < n; i++) {
        const a = base + j * (n + 1) + i;
        const b = a + 1;
        const c = a + n + 1;
        const d = c + 1;
        // winding so that normals point out: u x v == n for faces 0..5? use sign test below
        const sign = (f.u[1] as number) * (f.v[2] as number) - (f.u[2] as number) * (f.v[1] as number);
        void sign;
        idx.push(a, b, d, a, d, c);
      }
    }
  });
  return { pos, nor, uv, idx };
}

export interface TexturedOptions {
  /** Embed these as glTF textures (PNG buffers). */
  baseColor?: Buffer;
  normal?: Buffer;
  occlusion?: Buffer;
  /** glTF metallicRoughness texture (G = roughness, B = metallic). */
  metalRough?: Buffer;
  name?: string;
}

/** Build a glTF document: one node/mesh/primitive/material from MeshData, optional textures. */
export function buildDoc(m: MeshData, t: TexturedOptions = {}): Document {
  const doc = new Document();
  const buffer = doc.createBuffer('buffer');
  const acc = (name: string, type: 'VEC3' | 'VEC2' | 'SCALAR', arr: Float32Array | Uint16Array | Uint32Array) => doc.createAccessor(name).setType(type).setArray(arr).setBuffer(buffer);
  const prim = doc
    .createPrimitive()
    .setAttribute('POSITION', acc('pos', 'VEC3', new Float32Array(m.pos)))
    .setAttribute('NORMAL', acc('nor', 'VEC3', new Float32Array(m.nor)))
    .setAttribute('TEXCOORD_0', acc('uv', 'VEC2', new Float32Array(m.uv)))
    .setIndices(acc('idx', 'SCALAR', m.pos.length / 3 <= 65535 ? new Uint16Array(m.idx) : new Uint32Array(m.idx)));
  const mat: Material = doc.createMaterial(t.name ?? 'mat').setBaseColorFactor([1, 1, 1, 1]);
  const tex = (name: string, data: Buffer) => doc.createTexture(name).setImage(new Uint8Array(data)).setMimeType('image/png');
  if (t.baseColor) mat.setBaseColorTexture(tex('albedo', t.baseColor));
  if (t.normal) mat.setNormalTexture(tex('normal', t.normal));
  if (t.occlusion) mat.setOcclusionTexture(tex('ao', t.occlusion));
  if (t.metalRough) mat.setMetallicRoughnessTexture(tex('mr', t.metalRough));
  prim.setMaterial(mat);
  const mesh = doc.createMesh('m').addPrimitive(prim);
  const node = doc.createNode('n').setMesh(mesh);
  doc.createScene('s').addChild(node);
  return doc;
}

export async function writeGltf(doc: Document, path: string): Promise<void> {
  await new NodeIO().write(path, doc);
}
