import { describe, expect, it } from 'vitest';
import { cleanRing, extrudeBuilding, newArrays, signedArea, triangulate, type BuildingStats } from './building-mesh';

const sq = (x: number, z: number, s: number): [number, number][] => [[x, z], [x + s, z], [x + s, z + s], [x, z + s], [x, z]];
const triArea = (r: readonly (readonly [number, number])[], t: [number, number, number]): number => {
  const [a, b, c] = [r[t[0]]!, r[t[1]]!, r[t[2]]!];
  return Math.abs((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) / 2;
};

describe('triangulate', () => {
  it('covers a square exactly, either winding', () => {
    const r = cleanRing(sq(0, 0, 10));
    for (const ring of [r, [...r].reverse()]) {
      const t = triangulate(ring)!;
      expect(t).toHaveLength(2);
      expect(t.reduce((s, x) => s + triArea(ring, x), 0)).toBeCloseTo(100, 6);
    }
  });
  it('covers an L-shaped (concave) polygon: area preserved', () => {
    const L: [number, number][] = [[0, 0], [10, 0], [10, 4], [4, 4], [4, 10], [0, 10]];
    const t = triangulate(L)!;
    expect(t.reduce((s, x) => s + triArea(L, x), 0)).toBeCloseTo(Math.abs(signedArea(L)), 6);
    expect(Math.abs(signedArea(L))).toBe(64);
  });
  it('rejects degenerate input', () => {
    expect(triangulate([[0, 0], [1, 1]])).toBeNull();
    expect(triangulate([[0, 0], [1, 1], [2, 2]])).toBeNull();
  });
});

describe('extrudeBuilding', () => {
  it('builds walls + roof with outward normals and the right heights', () => {
    const out = newArrays();
    const st: BuildingStats = { buildings: 0, triangles: 0, roofFailures: 0 };
    extrudeBuilding({ ring: sq(0, 0, 8), levels: 2, style: 'x', enterable: false }, (x) => 5 + x * 0.1, out, st);
    expect(st).toEqual({ buildings: 1, triangles: 4 * 2 + 2, roofFailures: 0 });
    const ys = out.positions.filter((_, i) => i % 3 === 1);
    expect(Math.min(...ys)).toBeCloseTo(5 - 0.4, 6); // lowest ground 5.0
    expect(Math.max(...ys)).toBeCloseTo(5.8 + 6, 6); // highest ground 5.8 + 2 levels * 3 m
    // wall normals point away from the centre (4,4)
    for (let v = 0; v < out.positions.length / 3; v++) {
      const nx = out.normals[v * 3]!;
      const nz = out.normals[v * 3 + 2]!;
      if (nx === 0 && nz === 0) continue;
      const rx = out.positions[v * 3]! - 4;
      const rz = out.positions[v * 3 + 2]! - 4;
      expect(nx * rx + nz * rz).toBeGreaterThan(0);
    }
    expect(out.indices.every((i) => i < out.positions.length / 3)).toBe(true);
  });
  it('accepts clockwise input', () => {
    const out = newArrays();
    extrudeBuilding({ ring: [...sq(0, 0, 8)].reverse(), levels: 1, style: 'x', enterable: false }, () => 0, out);
    expect(out.indices.length).toBeGreaterThan(0);
  });
});
