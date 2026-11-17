import { Color4 } from '@babylonjs/core/Maths/math.color';
import { PERF_CAMERAS } from '../perf-cameras';
import { Matrix, Vector3 } from '@babylonjs/core/Maths/math.vector';
import type { UniversalCamera } from '@babylonjs/core/Cameras/universalCamera';
import type { RenderContext } from '../engine';
import { Terrain, parseTerrainDebug } from './terrain';
import { loadTerrainData } from './terrain-data';

export interface TerrainHandle {
  terrain: Terrain;
  /** Stop the render loop (e2e: screenshots of a software-rendered page are slow while frames keep coming). */
  pause(): void;
  /** Update the terrain for the current camera and render exactly one frame (use after `pause`). */
  renderOnce(): void;
  /** Move the fly camera (world metres). */
  setPose(pos: [number, number, number], target: [number, number, number]): void;
  /** Screen position (CSS px of the page, i.e. screenshot pixels) of the terrain surface point above world (x,z); for e2e probes. */
  project(x: number, z: number): { x: number; y: number; depth: number };
  loadMs: number;
  /** Overlay lines (nodes per LOD, select time, upload size). */
  overlayLines(): string[];
}

function triple(v: string | null): [number, number, number] | null {
  if (!v) return null;
  const p = v.split(',').map(Number);
  return p.length === 3 && p.every(Number.isFinite) ? [p[0]!, p[1]!, p[2]!] : null;
}

/**
 * Load the map height data and add the CDLOD terrain to the render context's scene.
 * Query: `terrain_debug=lod|morph|normal|height|flat`, `cam=0|1|2`, `pos=x,y,z`, `target=x,y,z`, `speed=m/frame`,
 * `terrain_yscale=k` (debug vertical exaggeration), `map=<id>` (default m0; files from `/maps/<id>/`, see DEVELOPING.md).
 */
export async function attachTerrain(ctx: RenderContext, params: URLSearchParams): Promise<TerrainHandle> {
  const t0 = performance.now();
  const mapId = params.get('map') ?? 'm0';
  const data = await loadTerrainData(`${import.meta.env.BASE_URL}maps/${mapId}/`);
  const debug = parseTerrainDebug(params.get('terrain_debug'));
  const terrain = new Terrain(ctx.scene, data, { kind: ctx.kind, debug, stitch: params.get('terrain_stitch') !== '0' });
  if (debug === 'flat') ctx.scene.clearColor = new Color4(1, 0, 1, 1);

  const yScale = Number(params.get('terrain_yscale') ?? 1);
  if (yScale !== 1) terrain.setYScale(yScale);
  const cam = ctx.scene.activeCamera as UniversalCamera;
  cam.maxZ = 12000;
  cam.minZ = 0.1;
  cam.speed = Number(params.get('speed') ?? 0.8);
  const preset = PERF_CAMERAS[Number(params.get('cam') ?? 0)] ?? PERF_CAMERAS[0]!;
  const pos = triple(params.get('pos')) ?? ([...preset.pos] as [number, number, number]);
  const target = triple(params.get('target')) ?? ([...preset.target] as [number, number, number]);
  if (pos[1] === 0) pos[1] = terrain.heightAt(pos[0], pos[2]) + 1.7;
  if (target[1] === 0) target[1] = terrain.heightAt(Math.min(Math.max(target[0], -3900), 4100), Math.min(Math.max(target[2], -3900), 4100)) + 1.5;
  cam.position = new Vector3(pos[0], pos[1], pos[2]);
  cam.setTarget(new Vector3(target[0], target[1], target[2]));

  ctx.onFrame(() => terrain.update(cam), undefined, 'terrain_update');
  terrain.update(cam); // so stats/first-update time exist before the first frame is rendered

  const handle: TerrainHandle = {
    terrain,
    pause() {
      ctx.engine.stopRenderLoop();
    },
    renderOnce() {
      terrain.update(cam);
      ctx.scene.render();
    },
    setPose(p, t) {
      cam.position = new Vector3(p[0], p[1], p[2]);
      cam.setTarget(new Vector3(t[0], t[1], t[2]));
    },
    project(x, z) {
      const eng = ctx.engine;
      const vp = cam.viewport.toGlobal(eng.getRenderWidth(), eng.getRenderHeight());
      const v = Vector3.Project(new Vector3(x, terrain.heightAt(x, z), z), Matrix.IdentityReadOnly, cam.getTransformationMatrix(), vp);
      const r = ctx.canvas.getBoundingClientRect(); // the canvas sits inside the body margin
      return { x: r.left + (v.x * r.width) / eng.getRenderWidth(), y: r.top + (v.y * r.height) / eng.getRenderHeight(), depth: v.z };
    },
    loadMs: performance.now() - t0,
    overlayLines() {
      const s = terrain.stats;
      return [
        `terrain   nodes ${s.nodes} (cut ${s.leaves})  tris ${s.triangles}  select ${s.selectMs.toFixed(3)} ms`,
        `  per LOD ${s.perLod.join(' ')}   tex ${(s.textureBytes / 1048576).toFixed(2)} MB  load ${handle.loadMs.toFixed(0)} ms`,
      ];
    },
  };
  return handle;
}
