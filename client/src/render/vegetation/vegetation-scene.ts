import { Matrix, Vector3 } from '@babylonjs/core/Maths/math.vector';
import type { Mesh } from '@babylonjs/core/Meshes/mesh';
import type { RenderContext } from '../engine';
import { parseQuality } from '../quality-m0';
import { Vegetation, type VegetationOptions, type VegetationStats } from './vegetation';
import type { ScatterPoint } from './scatter';
import { loadVegMask } from '../world/veg-mask';

export interface VegetationHandle {
  veg: Vegetation;
  loadMs: number;
  /** `{ drawCalls, instances, tris }` of the vegetation meshes that passed culling in the last frame (for `__perf.dump()`). */
  perf(): { drawCalls: number; instances: number; tris: number };
  overlayLines(): string[];
  /** Unsubscribe the frame callback and release all vegetation resources. Safe to call twice. */
  dispose(): void;
  /** Project a world point to page pixels (same convention as `__terrain.project`). */
  project(x: number, y: number, z: number): { x: number; y: number; depth: number };
}

interface SpawnsFile {
  hq?: Record<string, { pos: [number, number] }>;
  patrol_points?: { pos: [number, number] }[];
}

async function loadSpawnPoints(baseUrl: string): Promise<ScatterPoint[]> {
  try {
    const r = await fetch(`${baseUrl}spawns.json`);
    if (!r.ok) return [];
    const j = (await r.json()) as SpawnsFile;
    const pts: ScatterPoint[] = [];
    for (const h of Object.values(j.hq ?? {})) pts.push({ x: h.pos[0], z: h.pos[1] });
    for (const p of j.patrol_points ?? []) pts.push({ x: p.pos[0], z: p.pos[1] });
    return pts;
  } catch {
    return [];
  }
}

/**
 * Scatter + bake + attach vegetation to a render context that already has a terrain (`?scene=terrain`).
 * Entry: `?scene=vegetation` (terrain + vegetation; `?scene=terrain` stays terrain only). Query: `quality=high|low`, `impostor=lowres` (64 px views), `veg_seed=n`, `veg_budget=ms` (re-bucket budget, `inf` = unbounded).
 */
export async function attachVegetation(
  ctx: Pick<RenderContext, 'scene' | 'kind' | 'onFrame' | 'engine' | 'canvas'>,
  terrain: { heightAt(x: number, z: number): number },
  params: URLSearchParams,
): Promise<VegetationHandle> {
  const t0 = performance.now();
  const mapId = params.get('map') ?? 'm0';
  const base = `${import.meta.env.BASE_URL}maps/${mapId}/`;
  const spawns = await loadSpawnPoints(base);
  const meta = (await (await fetch(`${base}meta.json`)).json()) as { crs: { size_m: number } };
  const blockSizeM = meta.crs.size_m;
  const seed = Number(params.get('veg_seed') ?? 1) | 0;
  // Non-M0 maps (Carentan, M1-051b): keep vegetation off buildings/roads/hedges/water and put trees in forests; thin the grass
  // (`veg_grass`, default 0.4 [tbd]) and draw 4 x 4 chunks per mesh set (`veg_group`) to bound memory and per-draw CPU.
  const mask = mapId === 'm0' || params.get('veg_mask') === '0' ? null : await loadVegMask(base, blockSizeM);
  const grassScale = mask ? Number(params.get('veg_grass') ?? 0.4) : 1;
  const groupChunks = Number(params.get('veg_group') ?? (mask ? 4 : 1)) | 0;
  const budget = params.get('veg_budget');
  const opts: VegetationOptions = {
    kind: ctx.kind,
    quality: parseQuality(params.get('quality')),
    groupChunks,
    cellPx: params.get('impostor') === 'lowres' ? 64 : 128,
    rebucketBudgetMs: budget === 'inf' ? Infinity : budget ? Number(budget) : undefined,
  };
  const initialCamera = ctx.scene.activeCamera;
  if (!initialCamera) throw new Error('vegetation: an active camera is required');
  const veg = mapId === 'm0'
    ? await Vegetation.create(ctx.scene, terrain, Vegetation.scatterTables(blockSizeM, seed, terrain, spawns), opts)
    : await Vegetation.createProcedural(ctx.scene, terrain, { sizeM: blockSizeM, seed, exclusions: spawns, mask: mask ?? undefined, grassScale }, opts);
  try {
    veg.settle(ctx.scene.activeCamera ?? initialCamera);
  } catch (error) {
    veg.dispose();
    throw error;
  }
  const detach = ctx.onFrame((dtMs) => {
    const camera = ctx.scene.activeCamera;
    if (camera) veg.update(camera, dtMs / 1000);
  }, undefined, 'vegetation_update');
  let disposed = false;
  const handle: VegetationHandle = {
    veg,
    loadMs: performance.now() - t0,
    perf: () => veg.perfStats(),
    overlayLines: () => veg.overlayLines(),
    dispose() {
      if (disposed) return;
      disposed = true;
      ctx.scene.onDisposeObservable.remove(sceneDisposal);
      detach();
      veg.dispose();
    },
    project(x, y, z) {
      const cam = ctx.scene.activeCamera ?? initialCamera;
      const eng = ctx.engine;
      const vp = cam.viewport.toGlobal(eng.getRenderWidth(), eng.getRenderHeight());
      const v = Vector3.Project(new Vector3(x, y, z), Matrix.IdentityReadOnly, cam.getTransformationMatrix(), vp);
      const r = ctx.canvas.getBoundingClientRect();
      return { x: r.left + (v.x * r.width) / eng.getRenderWidth(), y: r.top + (v.y * r.height) / eng.getRenderHeight(), depth: v.z };
    },
  };
  const sceneDisposal = ctx.scene.onDisposeObservable.addOnce(() => handle.dispose());
  return handle;
}

export type { VegetationStats, Mesh };
