import { afterEach, expect, it, vi } from 'vitest';
import { FreeCamera } from '@babylonjs/core/Cameras/freeCamera';
import { NullEngine } from '@babylonjs/core/Engines/nullEngine';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { Scene } from '@babylonjs/core/scene';
import type { RenderContext } from '../engine';
import { attachVegetation } from './vegetation-scene';

const engines: NullEngine[] = [];
afterEach(() => {
  vi.unstubAllGlobals();
  for (const engine of engines.splice(0)) engine.dispose();
});

it.each(['m0', 'carentan'])('attaches %s and detaches the frame callback on teardown', async (map) => {
  const engine = new NullEngine();
  engines.push(engine);
  const scene = new Scene(engine);
  const camera = new FreeCamera('start', new Vector3(32, 2, 32), scene);
  const params = new URLSearchParams({ map, quality: 'low', veg_mask: '0', impostor: 'lowres', veg_budget: 'inf' });
  vi.stubGlobal('fetch', vi.fn(async (url: string) => new Response(JSON.stringify(url.endsWith('meta.json') ? { crs: { size_m: 64 } } : {}))));
  let onFrame: Parameters<RenderContext['onFrame']>[0] | undefined;
  const detach = vi.fn(() => { onFrame = undefined; });
  const ctx: Parameters<typeof attachVegetation>[0] = {
    kind: 'webgl2', scene, engine, canvas: {} as HTMLCanvasElement,
    onFrame: (cb) => { onFrame = cb; return detach; },
  };
  const handle = await attachVegetation(ctx, { heightAt: () => 2 }, params);
  expect(handle.veg.qualityTier).toBe('low');
  expect(handle.veg.stats.residentGroups).toBe(1);
  expect(handle.veg.stats.atlasBytes).toBeGreaterThan(0);
  expect(engine.scenes).toHaveLength(3); // Main scene plus one atlas owner per woody species.

  scene.activeCamera = new FreeCamera('far', new Vector3(3000, 2, 32), scene);
  onFrame?.(16);
  expect(handle.veg.stats.residentGroups).toBe(map === 'm0' ? 1 : 0);
  expect(handle.veg.stats.perLod.grass).toEqual([0, 0]);
  scene.activeCamera = camera;
  onFrame?.(16);
  expect(handle.veg.stats.residentGroups).toBe(1);

  // Exercise both the public API and the fallback when the render context is disposed first.
  if (map === 'm0') handle.dispose();
  else scene.dispose();
  handle.dispose();
  expect(detach).toHaveBeenCalledTimes(1);
  expect(onFrame).toBeUndefined();
  expect(handle.veg.meshes).toHaveLength(0);
  expect(handle.veg.atlases.size).toBe(0);
  expect(handle.veg.stats.atlasBytes).toBe(0);
  expect(handle.veg.stats.bufferBytes).toBe(0);
  expect(handle.perf()).toEqual({ drawCalls: 0, instances: 0, tris: 0 });
  expect(engine.scenes).toEqual(map === 'm0' ? [scene] : []);
});
