import { FixedStepper } from './game';
import { createRenderer, parseRendererParam, DebugOverlay, buildPerfDump, installPerfApi } from './render';
import type { PerfApi } from './render';
import type { TerrainHandle } from './render/terrain/terrain-scene';

declare global {
  interface Window {
    __perf?: PerfApi;
    __smoke?: unknown;
    __caps?: unknown;
    /** `?scene=terrain`: terrain diagnostics for e2e/M0-030 (stats, CPU height query, last node selection). */
    __terrain?: {
      stats: () => unknown;
      heightAt: (x: number, z: number) => number;
      nodes: () => number[][];
      loadMs: number;
      project: (x: number, z: number) => { x: number; y: number; depth: number };
      setPose: (pos: [number, number, number], target: [number, number, number]) => void;
      setStitch: (on: boolean) => void;
      setYScale: (v: number) => void;
      setDebug: (mode: string) => void;
    };
  }
}

/** Composition root: probe -> engine -> test scene -> loop. */
async function boot(): Promise<void> {
  const params = new URLSearchParams(location.search);
  const app = document.getElementById('app');
  if (!app) throw new Error('#app missing');
  const canvas = document.createElement('canvas');
  canvas.id = 'game-canvas';
  canvas.style.cssText = 'display:block;width:100vw;height:100vh;outline:none;touch-action:none';
  app.appendChild(canvas);

  const terrainScene = params.get('scene') === 'terrain';
  const ctx = await createRenderer(canvas, {
    renderer: parseRendererParam(params.get('renderer')),
    fly: params.get('fly') === '1' || terrainScene,
    empty: params.get('empty') === '1' || terrainScene,
  });
  let terrainHandle: TerrainHandle | null = null;
  if (terrainScene) {
    const { attachTerrain } = await import('./render/terrain/terrain-scene');
    terrainHandle = await attachTerrain(ctx, params);
    const t = terrainHandle.terrain;
    window.__terrain = { stats: () => JSON.parse(JSON.stringify(t.stats)) as unknown, heightAt: (x, z) => t.heightAt(x, z), nodes: () => t.debugNodes(), loadMs: terrainHandle.loadMs, project: (x, z) => terrainHandle!.project(x, z), setPose: (p, tg) => terrainHandle!.setPose(p, tg), setStitch: (on) => t.setStitch(on), setYScale: (v) => t.setYScale(v), setDebug: (m) => t.setDebug(m) };
  }

  const overlay = new DebugOverlay(params.get('debug') === '1');
  const stepper = new FixedStepper();
  const getDump = () =>
    buildPerfDump(ctx.snapshot(), { steps_total: stepper.totalSteps, stepper_clamped: stepper.stepperClamped });
  installPerfApi(window, getDump);

  ctx.onFrame((dt) => {
    const { steps } = stepper.advance(dt);
    void steps; // game.update(steps) / render interpolation (alpha) are wired by later tasks.
    overlay.update(performance.now(), getDump(), terrainHandle?.overlayLines());
  });
  // ?norender=1 skips the render loop (debug aid; also needed where WebGPU cannot present, e.g. SwiftShader).
  if (params.get('norender') !== '1') ctx.start();

  if (params.get('smoke') === 'compute') {
    const { runComputeSmoke } = await import('./render/compute-smoke');
    window.__smoke = await runComputeSmoke(ctx);
  }
}

boot().catch((e: unknown) => {
  console.error('[boot] failed', e);
  const app = document.getElementById('app');
  if (app) app.textContent = `Boot failed: ${e instanceof Error ? e.message : String(e)}`;
});
