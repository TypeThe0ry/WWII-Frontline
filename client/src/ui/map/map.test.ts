import { describe, it, expect } from 'vitest';
import { worldToGrid, gridToWorld, formatGrid, parseGrid, CELL_M, MAP_SIZE_M, fitZoom, zoomAt, screenToWorld, worldToScreen, clampView, gridDetail } from './coords';
import { MarkerBook, MARKER_TTL_S, markerRemaining } from '../../game/markers';
import { MapScheduler, unitStaleness } from './map-redraw';

describe('grid', () => {
  it('cell size and basic refs (A1 = NW corner, J10 = SE)', () => {
    expect(CELL_M).toBeCloseTo(204.8);
    expect(formatGrid(worldToGrid(1, 2047))).toBe('A1');
    expect(formatGrid(worldToGrid(2047, 1))).toBe('J10');
    expect(formatGrid(worldToGrid(2048, 0))).toBe('J10');
  });
  it('C4-7-3 sample: C4 cell, upper-left third, then its lower-right third', () => {
    const g = parseGrid('C4-7-3')!;
    expect(g).toEqual({ col: 2, row: 3, sub: [7, 3] });
    const w = gridToWorld(g);
    expect(formatGrid(worldToGrid(w.x, w.z, 2))).toBe('C4-7-3');
    // C4 spans x 409.6–614.4 and rows 3 (z from north): 614.4–819.2 from north → z 1228.8–1433.6.
    expect(w.x).toBeGreaterThan(409.6);
    expect(w.x).toBeLessThan(409.6 + CELL_M / 3);
    expect(w.z).toBeGreaterThan(1433.6 - CELL_M / 3);
  });
  it('round trip over 1000 random points is within 0.5 m at level 2 (cell 22.76 m → ≤ 11.4 m is the best guarantee, so use level 4)', () => {
    let seed = 12345;
    const rnd = () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 2 ** 32);
    let maxErr = 0;
    for (let i = 0; i < 1000; i++) {
      const x = rnd() * MAP_SIZE_M, z = rnd() * MAP_SIZE_M;
      const c = gridToWorld(worldToGrid(x, z, 6));
      maxErr = Math.max(maxErr, Math.hypot(c.x - x, c.z - z));
    }
    expect(maxErr).toBeLessThanOrEqual(0.5);
  });
  it('parse rejects bad refs, accepts A10', () => {
    expect(parseGrid('K1')).toBeNull();
    expect(parseGrid('C4-0')).toBeNull();
    expect(parseGrid('a10')).toEqual({ col: 0, row: 9, sub: [] });
  });
});

describe('view transform', () => {
  const vw = 1920, vh = 1080;
  it('zoom keeps the world point under the cursor fixed and clamps to 1×–8×', () => {
    const base = fitZoom(vw, vh);
    let v = clampView({ zoom: base, cx: 1024, cz: 1024 }, vw, vh);
    const sx = 1000, sy = 560;
    const before = screenToWorld(v, vw, vh, sx, sy);
    v = zoomAt(v, vw, vh, sx, sy, 2);
    const after = screenToWorld(v, vw, vh, sx, sy);
    expect(after.x).toBeCloseTo(before.x, 6);
    expect(after.z).toBeCloseTo(before.z, 6);
    for (let i = 0; i < 10; i++) v = zoomAt(v, vw, vh, sx, sy, 2);
    expect(v.zoom).toBeCloseTo(base * 8);
    expect(clampView({ zoom: 0.0001, cx: 0, cz: 0 }, vw, vh).zoom).toBeCloseTo(base);
  });
  it('pan stays inside the map; world↔screen inverse', () => {
    const base = fitZoom(vw, vh) * 4;
    const v = clampView({ zoom: base, cx: -500, cz: 9999 }, vw, vh);
    expect(v.cx).toBeGreaterThanOrEqual(vw / 2 / v.zoom - 1e-9);
    expect(v.cz).toBeLessThanOrEqual(MAP_SIZE_M - vh / 2 / v.zoom + 1e-9);
    const s = worldToScreen(v, vw, vh, 700, 800);
    const w = screenToWorld(v, vw, vh, s.sx, s.sy);
    expect(w.x).toBeCloseTo(700);
    expect(w.z).toBeCloseTo(800);
  });
  it('subdivision detail appears with zoom', () => {
    expect(gridDetail(fitZoom(vw, vh))).toBe(0);
    expect(gridDetail(fitZoom(vw, vh) * 4)).toBe(1);
    expect(gridDetail(fitZoom(vw, vh) * 8)).toBe(2);
  });
});

describe('markers', () => {
  it('4th Attention replaces the owner\'s oldest; others unaffected', () => {
    const b = new MarkerBook();
    const ids = [0, 1, 2].map((i) => b.place(1, 'attention', i, 0, i)!.id);
    b.place(2, 'attention', 0, 0, 3);
    b.place(1, 'attention', 9, 9, 4);
    const mine = b.list.filter((m) => m.owner === 1).map((m) => m.id);
    expect(mine).toHaveLength(3);
    expect(mine).not.toContain(ids[0]);
    expect(b.list.filter((m) => m.owner === 2)).toHaveLength(1);
  });
  it('orders need Officer; expiry follows TTL table', () => {
    const b = new MarkerBook();
    expect(b.place(1, 'orderAttack', 0, 0, 0)).toBeNull();
    const o = b.place(1, 'orderAttack', 0, 0, 0, true)!;
    const a = b.place(1, 'attention', 0, 0, 0)!;
    expect(b.expire(14.9)).toBe(0);
    expect(b.expire(15)).toBe(1);
    expect(b.list.map((m) => m.id)).toEqual([o.id]);
    expect(markerRemaining(a, 7.5)).toBeCloseTo(0.5);
    expect(MARKER_TTL_S.enemyVehicle).toBe(40);
    expect(b.expire(300)).toBe(1);
  });
});

describe('redraw scheduler', () => {
  it('no base bake while idle (10 s), none during drag, exactly one after release', () => {
    const s = new MapScheduler(1000 / 30);
    let t = 0;
    s.frame(t, true); // initial bake
    expect(s.stats.baseBakes).toBe(1);
    for (t = 16; t < 10_000; t += 16) s.frame(t, true);
    expect(s.stats.baseBakes).toBe(1);
    s.dragStart();
    for (let i = 0; i < 30; i++) { s.viewChanged(); s.frame((t += 16), true); }
    expect(s.stats.baseBakes).toBe(1);
    s.dragEnd();
    s.frame((t += 16), true);
    s.frame(t + 16, true);
    expect(s.stats.baseBakes).toBe(2);
  });
  it('dynamic layer capped at 30 fps and paused when hidden', () => {
    const s = new MapScheduler(1000 / 30);
    for (let t = 0; t < 1000; t += 1000 / 60) s.frame(t, true);
    expect(s.stats.dynamicDraws).toBeLessThanOrEqual(31);
    const n = s.stats.dynamicDraws;
    for (let t = 1000; t < 2000; t += 16) s.frame(t, false);
    expect(s.stats.dynamicDraws).toBe(n);
  });
  it('unit staleness after 5 s', () => {
    expect(unitStaleness(0, 5)).toEqual({ stale: false, agoS: 5 });
    expect(unitStaleness(0, 7.9)).toEqual({ stale: true, agoS: 7 });
  });
});
