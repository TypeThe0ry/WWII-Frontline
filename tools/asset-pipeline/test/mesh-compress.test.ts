import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Document, NodeIO } from '@gltf-transform/core';
import { describe, expect, it } from 'vitest';
import { compressMesh, readGlb, sceneBounds, writeGlb } from '../src/mesh-compress.ts';
import { countTriangles } from '../src/lod.ts';
import { albedoPng, buildDoc, sphereData } from './synth.ts';

describe('compressMesh (acceptance 5)', () => {
  it('quantizes pos16 / uv16 / normal8, keeps the bbox diagonal error < 0.1 %, meshopt round-trips', async () => {
    // Asymmetric, off-origin geometry makes the quantization range matter.
    const m = sphereData(60, 60, 3.7);
    for (let i = 0; i < m.pos.length; i += 3) {
      m.pos[i] = m.pos[i]! * 1.8 + 12.34;
      m.pos[i + 2] = m.pos[i + 2]! * 0.6 - 7.1;
    }
    const doc = buildDoc(m, { baseColor: albedoPng(8) }); // a texture keeps the UVs alive through prune
    const before = sceneBounds(doc);
    const res = await compressMesh(doc);
    expect(res.bboxError).toBeLessThan(0.001);
    const prim = doc.getRoot().listMeshes()[0]!.listPrimitives()[0]!;
    const is16 = (t: number) => t === 5122 || t === 5123; // SHORT / UNSIGNED_SHORT
    expect(is16(prim.getAttribute('POSITION')!.getComponentType())).toBe(true);
    expect(prim.getAttribute('POSITION')!.getNormalized()).toBe(true);
    expect(is16(prim.getAttribute('TEXCOORD_0')!.getComponentType())).toBe(true);
    expect(prim.getAttribute('NORMAL')!.getComponentType()).toBe(5120); // BYTE
    expect(doc.getRoot().listExtensionsRequired().map((e) => e.extensionName)).toContain('EXT_meshopt_compression');

    const dir = mkdtempSync(join(tmpdir(), 'fl-mc-'));
    try {
      const bytes = await writeGlb(doc, join(dir, 'a.glb'));
      expect(bytes).toBeGreaterThan(0);
      const back = await readGlb(join(dir, 'a.glb'));
      expect(countTriangles(back).tris).toBe(countTriangles(doc).tris);
      const after = sceneBounds(back);
      for (let k = 0; k < 3; k++) {
        expect(Math.abs(after.min[k]! - before.min[k]!)).toBeLessThan(0.001 * Math.hypot(...before.max.map((v, i) => v - before.min[i]!)));
      }
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('the compressed glb is much smaller than the raw float geometry', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'fl-mc-'));
    try {
      const raw = await new NodeIO().writeBinary(buildDoc(sphereData(80, 80)));
      const packed = await writeGlb(await compressed(buildDoc(sphereData(80, 80))), join(dir, 'b.glb'));
      expect(packed).toBeLessThan(raw.byteLength * 0.4);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});

async function compressed<T extends Document>(d: T): Promise<T> {
  await compressMesh(d);
  return d;
}
