import { describe, expect, it, vi } from 'vitest';
import { MAP_SIZE_M } from './coords';
import { availableLayers, mapPoint, MapRenderer, measureLine, pickSpawn } from './map-renderer';
import type { MapFrame, MapSpawn } from './map-renderer';

const view = { zoom: 1, cx: 1024, cz: 1024 };
const spawn: MapSpawn = { id: 1, x: 1024, z: 1024, label: 'HQ', kind: 'hq', status: 'available' };

describe('canvas map data and rendering', () => {
  it('picks the nearest spawn within the glyph radius', () => {
    const neighbour = { ...spawn, id: 2, x: 1034 };
    expect(pickSpawn([spawn, neighbour], view, 600, 400, 308, 200)).toBe(neighbour);
    expect(pickSpawn([spawn], view, 600, 400, 300, 400)).toBeUndefined();
  });

  it('measures in world metres and uses a clockwise compass bearing', () => {
    const from = { x: 100, z: 100 };
    expect(measureLine({ from, to: { x: 100, z: 200 } })).toEqual({ metres: 100, bearing: 0 });
    expect(measureLine({ from, to: { x: 0, z: 100 } })).toEqual({ metres: 100, bearing: 270 });
    expect(measureLine({ from, to: { x: 400, z: 500 } }).metres).toBe(500);
    expect(mapPoint({ x: -50, z: 10000 })).toEqual({ x: 0, z: MAP_SIZE_M });
  });

  it('reuses static grid imagery for overlay-only updates', () => {
    function mockCanvas() {
      const ctx = {
        save: vi.fn(), beginPath: vi.fn(), rect: vi.fn(), clip: vi.fn(), fillRect: vi.fn(), restore: vi.fn(),
        clearRect: vi.fn(), drawImage: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(), stroke: vi.fn(), fillText: vi.fn(),
      };
      const canvas = { width: 600, height: 400, getContext: () => ({ ...ctx, canvas }), ownerDocument: { createElement: () => base.canvas } };
      return { canvas, ctx };
    }
    const base = mockCanvas();
    const display = mockCanvas();
    const renderer = new MapRenderer(display.canvas as unknown as HTMLCanvasElement);
    const frame: MapFrame = { data: {}, view, layers: availableLayers('full'), mode: 'full' };
    renderer.draw(frame);
    renderer.draw({ ...frame, data: { nowS: 10 } });
    expect(base.ctx.clearRect).toHaveBeenCalledTimes(1);
    expect(display.ctx.drawImage).toHaveBeenCalledTimes(2);
    renderer.draw({ ...frame, view: { ...view, zoom: 2 } });
    renderer.draw({ ...frame, layers: { ...frame.layers, grid: false } });
    expect(base.ctx.clearRect).toHaveBeenCalledTimes(3);
  });
});
