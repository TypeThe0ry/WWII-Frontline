import { expect, it } from 'vitest';
import map from '../../../../data/maps/carentan/building-interiors.json?raw';
import { appendWall, buildingAt, containsPoint, type BuildingLayouts } from './building-interiors';
import { newArrays } from './building-mesh';

const { buildings } = JSON.parse(map) as BuildingLayouts;

it('matches the oriented-cuboid AABB for every shared Carentan render wall', () => {
  let checked = 0;
  let maxError = 0;
  for (const building of buildings) {
    for (const w of building.walls) {
      const arrays = newArrays();
      appendWall(arrays, w, 'stone_townhouse');
      expect(arrays.indices).toHaveLength(36);
      const [ax, az] = w.a;
      const [bx, bz] = w.b;
      const l = Math.hypot(bx - ax, bz - az);
      const hx = (Math.abs(bx - ax) + w.width * Math.abs(bz - az) / l) / 2;
      const hz = (Math.abs(bz - az) + w.width * Math.abs(bx - ax) / l) / 2;
      const cx = (ax + bx) / 2, cz = (az + bz) / 2;
      const values = [[cx - hx, cx + hx], [w.bottom, w.top], [cz - hz, cz + hz]];
      for (let i = 0; i < 3; i++) {
        const row = arrays.positions.filter((_, index) => index % 3 === i);
        maxError = Math.max(maxError, Math.abs(Math.min(...row) - values[i]![0]!), Math.abs(Math.max(...row) - values[i]![1]!));
      }
      checked++;
    }
  }
  console.log('Shared building wall alignment:', { checked, maxError });
  expect(checked).toBeGreaterThan(100);
  expect(maxError).toBeLessThan(0.001);
});

it('classifies a concave room without filling its courtyard or space above the roof', () => {
  const b = buildings.find((room) => room.holes.length > 0)!;
  const probe = b.probes[0]!;
  const courtyard = [750, 821] as const;
  expect(containsPoint(b.ring, ...courtyard)).toBe(true);
  expect(containsPoint(b.holes[0]!, ...courtyard)).toBe(true);
  expect(buildingAt([b], ...probe)?.id).toBe(b.id);
  expect(buildingAt([b], courtyard[0], b.base_y + 1, courtyard[1])).toBeUndefined();
  expect(buildingAt([b], probe[0], b.roof_y, probe[2])).toBeUndefined();
});
