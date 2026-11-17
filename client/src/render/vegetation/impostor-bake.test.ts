import { describe, expect, it } from 'vitest';
import { ATLAS_CELLS, ATLAS_PX, CELL_PX, CELL_PX_LOWRES, cellCenter, cellOfDirection, hemiOctDecode, hemiOctEncode, impostorBasis } from './impostor-bake';

describe('octahedral impostor mapping', () => {
  it('atlas is 12 x 12 views of 128 px = 1536 px (64 px per view in lowres mode)', () => {
    expect(ATLAS_CELLS).toBe(12);
    expect(CELL_PX).toBe(128);
    expect(ATLAS_PX).toBe(1536);
    expect(ATLAS_CELLS * CELL_PX_LOWRES).toBe(768);
  });

  it('decode gives unit directions on the upper hemisphere and encode inverts it', () => {
    for (let ci = 0; ci < ATLAS_CELLS; ci++) {
      for (let cj = 0; cj < ATLAS_CELLS; cj++) {
        const [a, b] = cellCenter(ci, cj);
        const d = hemiOctDecode(a, b);
        expect(Math.hypot(...d)).toBeCloseTo(1, 9);
        expect(d[1]).toBeGreaterThan(0.05);
        const [a2, b2] = hemiOctEncode(...d);
        expect(a2).toBeCloseTo(a, 9);
        expect(b2).toBeCloseTo(b, 9);
        expect(cellOfDirection(...d)).toEqual([ci, cj]);
      }
    }
  });

  it('covers all 8 horizontal azimuths with distinct cells near the horizon', () => {
    const cells = new Set<string>();
    for (let k = 0; k < 8; k++) {
      const az = (k / 8) * Math.PI * 2;
      const e = (8 * Math.PI) / 180;
      cells.add(cellOfDirection(Math.cos(az) * Math.cos(e), Math.sin(e), Math.sin(az) * Math.cos(e)).join(','));
    }
    expect(cells.size).toBe(8);
  });

  it('clamps directions below the horizon to the horizon ring', () => {
    expect(cellOfDirection(0.6, -0.5, 0.8)).toEqual(cellOfDirection(0.6, 0, 0.8));
  });

  it('billboard basis is orthonormal, right-handed with the view direction and keeps up-ish', () => {
    for (const d of [[0, 0.1, 1], [1, 0.3, 0], [0.6, 0.5, -0.62], [0, 1, 0]] as const) {
      const n = Math.hypot(...d);
      const dn: [number, number, number] = [d[0] / n, d[1] / n, d[2] / n];
      const { right, up } = impostorBasis(dn);
      expect(Math.hypot(...right)).toBeCloseTo(1, 9);
      expect(Math.hypot(...up)).toBeCloseTo(1, 9);
      expect(right[0] * dn[0] + right[1] * dn[1] + right[2] * dn[2]).toBeCloseTo(0, 9);
      expect(up[0] * dn[0] + up[1] * dn[1] + up[2] * dn[2]).toBeCloseTo(0, 9);
      expect(right[0] * up[0] + right[1] * up[1] + right[2] * up[2]).toBeCloseTo(0, 9);
      if (dn[1] < 0.99) expect(up[1]).toBeGreaterThan(0);
    }
  });
});
