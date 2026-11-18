import { describe, expect, it } from 'vitest';
import hedgesRaw from '../../../../data/maps/carentan/hedgerows.geojson?raw';
import { blocksMovement, polylineToSegments, segmentAabb, segmentCorners, totalLength, useFarLod, type HedgeSegment } from './hedge-geometry';

const props = { height_m: 2, mound_width_m: 1.2 };

describe('polylineToSegments', () => {
  it('preserves total length and splits long edges to <= 6 m', () => {
    const segs = polylineToSegments([[0, 0], [30, 0], [30, 40]], props);
    expect(totalLength(segs)).toBeCloseTo(70);
    expect(Math.max(...segs.map((s) => s.len))).toBeLessThanOrEqual(6);
    expect(segs.length).toBe(5 + 7);
    expect(segs[0]!.yaw).toBeCloseTo(Math.PI / 2); // +x
    expect(segs.at(-1)!.yaw).toBeCloseTo(0); // +z
  });
  it('degenerate edges are skipped; gap flag maps to no-block', () => {
    expect(polylineToSegments([[1, 1], [1, 1]], props)).toEqual([]);
    const g = polylineToSegments([[0, 0], [3, 0]], { ...props, gap: true });
    expect(g[0]!.gap).toBe(true);
    expect(blocksMovement(g[0]!)).toBe(false);
    expect(blocksMovement(polylineToSegments([[0, 0], [3, 0]], props)[0]!)).toBe(true);
  });
});

describe('hedgerow_align: render AABB vs sim box', () => {
  it('200 random segments: AABB equals the bounds of the oriented box within 0.1 m (here exactly)', () => {
    let s = 3;
    const rnd = () => ((s = (s * 1664525 + 1013904223) >>> 0) / 2 ** 32);
    for (let i = 0; i < 200; i++) {
      const seg: HedgeSegment = {
        x: rnd() * 2048, z: rnd() * 2048, yaw: rnd() * Math.PI * 2, len: 1 + rnd() * 5, height: 2,
        moundWidth: 0.6 + rnd() * 1.4, gap: false, vehicleBlocking: true,
      };
      const a = segmentAabb(seg);
      const c = segmentCorners(seg);
      const xs = c.map((p) => p[0]);
      const zs = c.map((p) => p[1]);
      expect(Math.abs(a.minX - Math.min(...xs))).toBeLessThanOrEqual(0.1);
      expect(Math.abs(a.maxX - Math.max(...xs))).toBeLessThanOrEqual(0.1);
      expect(Math.abs(a.minZ - Math.min(...zs))).toBeLessThanOrEqual(0.1);
      expect(Math.abs(a.maxZ - Math.max(...zs))).toBeLessThanOrEqual(0.1);
    }
  });
  it('far LOD starts beyond 150 m', () => {
    expect(useFarLod(149)).toBe(false);
    expect(useFarLod(151)).toBe(true);
  });
});

describe('carentan hedgerow data in the M1 slice', () => {
  it('converts every feature; M1 length measured (the >4 km target is NOT met by the OSM draft)', () => {
    interface F { geometry: { type: string; coordinates: [number, number][] }; properties: { height_m: number; mound_width_m: number; gap: boolean; vehicle_blocking: boolean } }
    const feats = (JSON.parse(hedgesRaw) as { features: F[] }).features;
    let m1 = 0;
    let all = 0;
    for (const f of feats) {
      const segs = polylineToSegments(f.geometry.coordinates, f.properties);
      all += totalLength(segs);
      for (const s of segs) if (s.x >= 380 && s.x <= 1560 && s.z >= 520 && s.z <= 1700) m1 += s.len;
    }
    expect(feats.length).toBe(32);
    expect(Math.round(all)).toBeGreaterThan(2000);
    // Recorded as a fact for STATUS / PI-103: ~708 m inside the slice today.
    expect(m1).toBeGreaterThan(600);
    expect(m1).toBeLessThan(4000);
  });
});
