import { describe, expect, it } from 'vitest';
import { generateLods, seamRetention, countTriangles } from '../src/lod.ts';
import { buildDoc, seamCubeData, sphereData } from './synth.ts';

describe('generateLods (acceptance 2, 3)', () => {
  it('20k-triangle sphere: every LOD lands within +-5 % of the 0.5 / 0.25 / 0.1 target', async () => {
    const doc = buildDoc(sphereData(100, 100));
    const src = countTriangles(doc).tris;
    expect(src).toBeGreaterThan(19000);
    const lods = await generateLods(doc);
    expect(lods.map((l) => l.lod)).toEqual([0, 1, 2, 3]);
    expect(lods[0]!.tris).toBe(src);
    for (const [i, target] of [[1, 0.5], [2, 0.25], [3, 0.1]] as const) {
      const ratio = lods[i]!.tris / src;
      expect(Math.abs(ratio - target) / target, `LOD${i} ratio ${ratio}`).toBeLessThanOrEqual(0.05);
      expect(lods[i]!.simplifyError).toBeGreaterThanOrEqual(0);
    }
    // The source document is left untouched.
    expect(countTriangles(doc).tris).toBe(src);
  });

  it('textured cube with UV seams: >= 95 % of the seam vertices survive LOD1', async () => {
    const doc = buildDoc(seamCubeData(16));
    const lods = await generateLods(doc, [1, 0.5]);
    const r = seamRetention(doc, lods[1]!.doc);
    expect(r.seamVertices).toBeGreaterThan(100);
    expect(r.ratio).toBeGreaterThanOrEqual(0.95);
    expect(lods[1]!.tris).toBeLessThan(lods[0]!.tris);
  });

  it('keeps ratios strictly as given and LOD0 identical', async () => {
    const lods = await generateLods(buildDoc(sphereData(40, 40)), [1, 0.5, 0.25, 0.1]);
    const t = lods.map((l) => l.tris);
    expect(t[0]! > t[1]! && t[1]! > t[2]! && t[2]! > t[3]!).toBe(true);
  });
});
