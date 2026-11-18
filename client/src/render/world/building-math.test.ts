import { describe, expect, it } from 'vitest';
import raw from '../../../../data/maps/carentan/buildings.geojson?raw';
import {
  buildingLod, centroid, checkOpening, ENTERABLE_MAX_FRACTION, enterableFraction, probesFor, stairPlan, stairWalkable, wallAabb, type Ring,
} from './building-math';

describe('openings and stairs', () => {
  it('door minimums 1.0 x 2.0 m, flush sill; windows may have a sill', () => {
    expect(checkOpening({ type: 'door', wall: 0, offset: 1, w: 1.0, h: 2.0, sill: 0 }, 6)).toEqual([]);
    expect(checkOpening({ type: 'door', wall: 0, offset: 1, w: 0.9, h: 1.9, sill: 0.2 }, 6)).toEqual(['door too narrow', 'door too low', 'door sill above floor']);
    expect(checkOpening({ type: 'window', wall: 0, offset: 1, w: 1.2, h: 1.2, sill: 0.9 }, 6)).toEqual([]);
    expect(checkOpening({ type: 'window', wall: 0, offset: 5.5, w: 1.2, h: 1.2, sill: 0.9 }, 6)).toEqual(['opening outside wall']);
    expect(checkOpening({ type: 'window', wall: 0, offset: 1, w: 1, h: 2.5, sill: 1 }, 6)).toEqual(['opening above ceiling']);
  });
  it('stairs: <=35 deg ramp or <=0.4 m steps; the planned stair climbs one level', () => {
    expect(stairWalkable(3, 5)).toBe(true); // ~31 deg
    expect(stairWalkable(3, 3)).toBe(false); // 45 deg
    expect(stairWalkable(3, 3, 0.2)).toBe(true);
    const p = stairPlan();
    expect(p.steps).toBe(8);
    expect(3 / p.steps).toBeLessThanOrEqual(0.4);
  });
});

describe('probes and lod', () => {
  const ring: Ring = [[0, 0], [10, 0], [10, 6], [0, 6]];
  it('one probe per level at the centroid, 1.2 m above each floor', () => {
    expect(centroid(ring)).toEqual([5, 3]);
    const p = probesFor(ring, 3, 10);
    expect(p.length).toBe(3);
    expect(p.map((q) => q.y)).toEqual([11.2, 14.2, 17.2]);
    expect(probesFor(ring, 0).length).toBe(1);
  });
  it('lod thresholds 60/150/400, shell-only beyond', () => {
    expect([10, 60, 61, 150, 399, 400, 401, 900].map(buildingLod)).toEqual([0, 0, 1, 1, 2, 2, 3, 3]);
  });
});

describe('building_align', () => {
  it('wall AABB of an axis-aligned and a diagonal wall', () => {
    expect(wallAabb([0, 0], [10, 0], 0.4)).toEqual({ minX: 0, maxX: 10, minZ: -0.2, maxZ: 0.2 });
    const d = wallAabb([0, 0], [10, 10], 0.4);
    expect(d.maxX - d.minX).toBeCloseTo(10 + 0.4 * Math.SQRT1_2 * 1);
  });
});

describe('carentan data', () => {
  it('enterable fraction in the M1 slice is <= 25 % (measured 5.0 %: 70 of 1408)', () => {
    interface F { geometry: { coordinates: [number, number][][] }; properties: { enterable: boolean } }
    const feats = (JSON.parse(raw) as { features: F[] }).features;
    const flags: boolean[] = [];
    for (const f of feats) {
      const [cx, cz] = centroid(f.geometry.coordinates[0] as [number, number][]);
      if (cx >= 380 && cx <= 1560 && cz >= 520 && cz <= 1700) flags.push(f.properties.enterable);
    }
    expect(flags.length).toBe(1408);
    expect(flags.filter(Boolean).length).toBe(70);
    expect(enterableFraction(flags)).toBeLessThanOrEqual(ENTERABLE_MAX_FRACTION);
  });
});
