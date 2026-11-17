import { MeshoptDecoder, MeshoptEncoder } from 'meshoptimizer';
import { describe, expect, it } from 'vitest';
import { AssetLoader, type Manifest } from './index';
import { decodeMesh, decodeTexture } from './entry-decode';
import { inlineDecoder } from './worker-decoder';
import { handleDecode } from './decode-worker';

const align4 = (n: number) => Math.ceil(n / 4) * 4;

/** Encode one primitive (a quad: POSITION f32x3 + TEXCOORD_0 u16x2 normalized) following SPEC.md "Mesh entry layout v1". */
async function encodeQuad(): Promise<{ bytes: Uint8Array; positions: Float32Array }> {
  await MeshoptEncoder.ready;
  const positions = Float32Array.of(0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0);
  const uvs = Uint16Array.of(0, 0, 65535, 0, 65535, 65535, 0, 65535);
  const stride = 12 + 4; // position 12 B, uv 4 B (already 4-aligned)
  const inter = new Uint8Array(stride * 4);
  for (let v = 0; v < 4; v++) {
    inter.set(new Uint8Array(positions.buffer, v * 12, 12), v * stride);
    inter.set(new Uint8Array(uvs.buffer, v * 4, 4), v * stride + 12);
  }
  const vb = MeshoptEncoder.encodeVertexBuffer(inter, 4, stride);
  const ib = MeshoptEncoder.encodeIndexBuffer(Uint8Array.from(new Uint8Array(Uint16Array.of(0, 1, 2, 0, 2, 3).buffer)), 6, 2);
  const parts: number[] = [];
  const w = {
    u8: (v: number) => parts.push(v),
    u16: (v: number) => parts.push(v & 255, v >> 8),
    u32: (v: number) => parts.push(v & 255, (v >> 8) & 255, (v >> 16) & 255, (v >>> 24) & 255),
    f32: (v: number) => parts.push(...new Uint8Array(Float32Array.of(v).buffer)),
    pad: () => { while (parts.length % 4) parts.push(0); },
  };
  w.u8(1); w.u8(0); w.u16(1); w.u16(1); w.u16(0);
  [0, 0, 0, 1, 1, 0].forEach(w.f32); // bbox
  [0.5, 0.25, 0.125, 1].forEach(w.f32); w.f32(0.3); w.f32(0.7); [0, 0, 0].forEach(w.f32); // material
  const tex = ['prop_x@tex_albedo', '', '', ''];
  for (const t of tex) { w.u8(t.length); parts.push(...new TextEncoder().encode(t)); }
  w.pad();
  w.u16(0); w.u8(2); w.u8(2); w.u32(4); w.u32(6); w.u32(vb.length); w.u32(ib.length);
  [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1].forEach(w.f32);
  w.u8(0); w.u8(5); w.u8(3); w.u8(0); // POSITION f32 x3
  w.u8(3); w.u8(3); w.u8(2); w.u8(1); // TEXCOORD_0 u16 x2 normalized
  parts.push(...vb); w.pad();
  parts.push(...ib); w.pad();
  return { bytes: Uint8Array.from(parts), positions };
}

describe('entry decoders', () => {
  it('decodes a mesh entry: positions, uvs, indices, material and texture refs round-trip', async () => {
    const { bytes, positions } = await encodeQuad();
    const m = await decodeMesh(bytes, MeshoptDecoder);
    expect(m.materials[0]).toMatchObject({ metallic: expect.closeTo(0.3, 5), roughness: expect.closeTo(0.7, 5), textures: ['prop_x@tex_albedo', '', '', ''] });
    const p = m.primitives[0]!;
    expect(p.vertexCount).toBe(4);
    expect(Array.from(p.indices)).toEqual([0, 1, 2, 0, 2, 3]);
    const pos = p.attributes.find((a) => a.semantic === 'POSITION')!;
    expect(Array.from(new Float32Array(pos.data.buffer, pos.data.byteOffset, 12))).toEqual(Array.from(positions));
    const uv = p.attributes.find((a) => a.semantic === 'TEXCOORD_0')!;
    expect(uv.normalized).toBe(true);
    expect(Array.from(new Uint16Array(uv.data.buffer, uv.data.byteOffset, 8))).toEqual([0, 0, 65535, 0, 65535, 65535, 0, 65535]);
  });

  it('rejects an unknown layout version', async () => {
    await expect(decodeMesh(Uint8Array.of(9, 0, 0, 0, 0, 0, 0, 0), MeshoptDecoder)).rejects.toThrow(/layout version/);
    expect(() => decodeTexture(Uint8Array.of(9, ...new Array(60).fill(0)))).toThrow(/layout version/);
  });

  it('worker handler returns an error response instead of throwing', async () => {
    const r = await handleDecode({ id: 7, entryId: 'x@lod0', kind: 'mesh', bytes: Uint8Array.of(9) });
    expect(r).toMatchObject({ id: 7, ok: false });
  });

  it('AssetLoader with the inline decoder returns decoded geometry through the upload queue', async () => {
    const { bytes } = await encodeQuad();
    const manifest: Manifest = { schema: 1, game_version: 't', assets: { prop_x: { kind: 'model', tier: ['medium'], placeholder: false, lods: [{ lod: 0, suffix: '', pak: '/a/p.fpak', entry: 'prop_x@lod0', offset: 0, bytes: bytes.length, sha256: 'h' }] } } };
    const { CacheLayer } = await import('./index');
    const cache = new CacheLayer();
    await cache.put('h', bytes); // serve from cache: no network in this test
    const l = new AssetLoader({ manifest, cache, decoder: inlineDecoder, log: () => undefined });
    await l.init({ tier: 'medium' });
    const p = l.loadMesh('prop_x', 0);
    for (let i = 0; i < 20 && l.uploads.pending === 0; i++) await new Promise((r) => setTimeout(r, 0));
    l.uploads.runFrame();
    const r = (await p) as { data: Awaited<ReturnType<typeof decodeMesh>> };
    expect(r.data.primitives[0]!.vertexCount).toBe(4);
    expect(align4(5)).toBe(8);
  });
});

describe('texture transcoding in the decode path (M1-006c)', () => {
  it('returns transcoded GPU data when a transcoder is configured, parsed data otherwise', async () => {
    const { readFileSync } = await import('node:fs');
    const { createRequire } = await import('node:module');
    const { fileURLToPath } = await import('node:url');
    const { instantiateBasis } = await import('./transcode');
    const { basisTranscoder } = await import('./decode-worker');
    const { inlineDecoderWith, inlineDecoder } = await import('./worker-decoder');
    const dir = new URL('../../../public/basis/', import.meta.url);
    const load = async () => {
      const wasm = readFileSync(fileURLToPath(new URL('basis_transcoder.wasm', dir)));
      return instantiateBasis(readFileSync(fileURLToPath(new URL('basis_transcoder.js', dir)), 'utf8'), wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength) as ArrayBuffer, {
        require: createRequire(import.meta.url), __dirname: fileURLToPath(dir), __filename: fileURLToPath(new URL('basis_transcoder.js', dir)),
      });
    };
    const bytes = new Uint8Array(readFileSync(fileURLToPath(new URL('./fixtures/tex_etc1s_16.entry', import.meta.url))));
    const plain = await inlineDecoder.decode('t', 'texture', bytes);
    expect((plain.data as { vkFormat?: number }).vkFormat).toBeDefined();
    let loads = 0;
    const dec = inlineDecoderWith(basisTranscoder({ caps: { astc: true }, basisBase: '/basis' }, (b) => { loads++; expect(b).toBe('/basis'); return load(); }));
    for (let i = 0; i < 2; i++) {
      const out = await dec.decode('t', 'texture', bytes);
      expect(out.data).toMatchObject({ format: 'astc4x4', width: 16, height: 16 });
    }
    expect(loads).toBe(1); // module is loaded once per worker
  });
});
