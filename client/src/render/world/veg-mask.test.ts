import { describe, expect, it } from 'vitest';
import { buildVegMask } from './veg-mask';

const sq = (x: number, z: number, s: number): [number, number][] => [[x, z], [x + s, z], [x + s, z + s], [x, z + s]];

describe('veg mask', () => {
  const mask = buildVegMask({
    size: 100,
    buildings: [sq(10, 10, 10)],
    roads: [{ pts: [[0, 50], [100, 50]], width_m: 4 }],
    hedges: [{ pts: [[50, 0], [50, 40]], width_m: 1 }],
    waterPolys: [sq(70, 70, 10)],
    forests: [sq(20, 60, 20)],
  });
  it('blocks building footprint plus margin, not far away', () => {
    expect(mask.blocked(15, 15)).toBe(true);
    expect(mask.blocked(9, 15)).toBe(true); // 1 m outside: inside the 1.5 m margin
    expect(mask.blocked(7, 15)).toBe(false);
  });
  it('blocks road width and hedge line, leaves the verge free', () => {
    expect(mask.blocked(30, 50)).toBe(true);
    expect(mask.blocked(30, 51.9)).toBe(true); // cell centre 1.5 m from the axis
    expect(mask.blocked(30, 53.5)).toBe(false);
    expect(mask.blocked(50, 20)).toBe(true);
    expect(mask.blocked(52, 20)).toBe(false);
  });
  it('blocks water, marks forest and keeps forest cells unblocked', () => {
    expect(mask.blocked(75, 75)).toBe(true);
    expect(mask.forest(30, 70)).toBe(true);
    expect(mask.blocked(30, 70)).toBe(false);
    expect(mask.forest(5, 5)).toBe(false);
  });
  it('treats outside the grid as free and counts cells', () => {
    expect(mask.blocked(-5, 5)).toBe(false);
    expect(mask.counts.forest).toBe(400);
    expect(mask.counts.blocked).toBeGreaterThan(300);
  });
});
