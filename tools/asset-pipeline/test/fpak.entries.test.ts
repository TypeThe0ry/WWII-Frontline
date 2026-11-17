import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  assertNoStandalone, assertOpaque, bytesEqual, checkOpaque, decodeMeshEntry, decodeTextureEntry, encodeMeshEntry, encodeTextureEntry, findStandalone,
  meshEntryFromDocument, textureEntryFromKtx2,
} from '../src/fpak/index.ts';
import { compressMesh } from '../src/mesh-compress.ts';
import { decodePng, encodeKtx2, inspectKtx2, selectFormat } from '../src/textures.ts';
import { albedoPng, buildDoc, sphereData } from './synth.ts';

describe('mesh entry v1', () => {
  it('encode -> decode keeps every attribute, index and material byte for byte; bbox matches', async () => {
    const doc = buildDoc(sphereData(48, 48, 2), { baseColor: albedoPng(8) });
    doc.getRoot().listMaterials()[0]!.setBaseColorFactor([0.5, 0.25, 0.75, 1]).setRoughnessFactor(0.4).setMetallicFactor(0.9);
    await compressMesh(doc);
    const e = meshEntryFromDocument(doc, { textureId: () => 'prop_x@tex_albedo' });
    const bytes = await encodeMeshEntry(e);
    const d = await decodeMeshEntry(bytes);
    expect(d.primitives).toHaveLength(e.primitives.length);
    for (const [i, p] of e.primitives.entries()) {
      const q = d.primitives[i]!;
      expect(q.vertexCount).toBe(p.vertexCount);
      expect(q.indexSize).toBe(p.indexSize);
      expect(Array.from(q.indices)).toEqual(Array.from(p.indices));
      expect(q.matrix.map((x) => Math.fround(x))).toEqual(p.matrix.map((x) => Math.fround(x)));
      expect(q.attributes.map((a) => a.semantic)).toEqual(p.attributes.map((a) => a.semantic));
      for (const [k, a] of p.attributes.entries()) {
        expect(bytesEqual(q.attributes[k]!.data, a.data), a.semantic).toBe(true);
        expect(q.attributes[k]!.normalized).toBe(a.normalized);
      }
    }
    expect(d.materials[0]!.baseColor.map((x) => Math.round(x * 100) / 100)).toEqual([0.5, 0.25, 0.75, 1]);
    expect(d.materials[0]!.roughness).toBeCloseTo(0.4, 5);
    expect(d.materials[0]!.textures[0]).toBe('prop_x@tex_albedo');
    // Radius 2 sphere centred at the origin: |bbox| ~ 4.
    expect(d.bbox.max[0] - d.bbox.min[0]).toBeGreaterThan(3.9);
    expect(d.bbox.max[0] - d.bbox.min[0]).toBeLessThan(4.1);
    expect(bytes.length).toBeLessThan(doc.getRoot().listMeshes()[0]!.listPrimitives()[0]!.getAttribute('POSITION')!.getCount() * 20);
  });

  it('skinned / animated documents are refused', async () => {
    const doc = buildDoc(sphereData(8, 8));
    doc.createSkin('s');
    expect(() => meshEntryFromDocument(doc, { textureId: () => '' })).toThrow(/skinned/);
  });
});

describe('texture entry v1', () => {
  it.each([
    ['ETC1S', false],
    ['UASTC', true],
  ] as const)('%s KTX2 -> entry -> decode: header fields, DFD, global data and every mip identical', async (fmt, normal) => {
    const file = await encodeKtx2(decodePng(new Uint8Array(albedoPng(64))), { format: selectFormat(fmt === 'ETC1S' ? 'bg' : 'hero'), srgb: !normal, normal, channels: normal ? 2 : 3 });
    const t = textureEntryFromKtx2(file);
    const bytes = encodeTextureEntry(t);
    const d = decodeTextureEntry(bytes);
    expect(inspectKtx2(file).format).toBe(fmt);
    expect([d.width, d.height, d.vkFormat, d.supercompressionScheme, d.channels, d.layout]).toEqual([t.width, t.height, t.vkFormat, t.supercompressionScheme, t.channels, t.layout]);
    expect(bytesEqual(d.dfd, t.dfd)).toBe(true);
    expect(bytesEqual(d.sgd, t.sgd)).toBe(true);
    expect(d.levels).toHaveLength(t.levels.length);
    expect(t.levels.length).toBe(7);
    for (const [i, l] of t.levels.entries()) {
      expect(bytesEqual(d.levels[i]!.data, l.data), `mip ${i}`).toBe(true);
      expect(d.levels[i]!.uncompressedByteLength).toBe(l.uncompressedByteLength);
    }
    if (fmt === 'ETC1S') expect(t.sgd.length).toBeGreaterThan(0);
    // smallest mip is stored first
    const small = bytes.indexOf(t.levels[6]!.data[0]!);
    expect(small).toBeGreaterThan(0);
  });
});

describe('opaque entries (acceptance 5)', () => {
  it('no generic tool recognises mesh or texture entries', async () => {
    const doc = buildDoc(sphereData(16, 16), { baseColor: albedoPng(8) });
    await compressMesh(doc);
    const mesh = await encodeMeshEntry(meshEntryFromDocument(doc, { textureId: () => '' }));
    const tex = encodeTextureEntry(textureEntryFromKtx2(await encodeKtx2(decodePng(new Uint8Array(albedoPng(16))), { format: 'ETC1S', srgb: true })));
    for (const [name, b] of [['mesh', mesh], ['texture', tex]] as const) {
      const checks = await assertOpaque(b, name);
      expect(checks.length).toBeGreaterThanOrEqual(4);
    }
    // Control: a real KTX2 file and a real GLB are recognised, so the checks do bite.
    const ktx = await encodeKtx2(decodePng(new Uint8Array(albedoPng(16))), { format: 'ETC1S', srgb: true });
    expect((await checkOpaque(ktx)).some((c) => c.recognized)).toBe(true);
  });
});

describe('assert-no-standalone (acceptance 4)', () => {
  it('passes on a pak-only tree and fails once a .ktx2 (or other media file) is added', () => {
    const dir = mkdtempSync(join(tmpdir(), 'fl-sa-'));
    try {
      mkdirSync(join(dir, 'a', 'ab'), { recursive: true });
      writeFileSync(join(dir, 'a', 'ab', 'cdef.fpak'), 'x');
      writeFileSync(join(dir, 'packs.json'), '{}');
      expect(findStandalone(dir)).toEqual([]);
      expect(() => assertNoStandalone(dir)).not.toThrow();
      for (const f of ['t.ktx2', 'm.GLB', 'n.gltf', 's.ogg', 's.opus', 's.wav', 'i.png', 'i.jpg', 'i.jpeg']) {
        writeFileSync(join(dir, 'a', f), 'x');
        expect(() => assertNoStandalone(dir), f).toThrow(/standalone/);
        rmSync(join(dir, 'a', f));
      }
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
