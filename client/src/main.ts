import { EventBus, FixedStepper } from './game';
import type { GameEvents } from './game';
import type { NetGameHandle } from './game/net-game';
import type { ActorsHandle } from './actors-boot';
import { createRenderer, parseRendererParam, DebugOverlay, buildPerfDump, installPerfApi } from './render';
import type { PerfApi } from './render';
import type { TerrainHandle } from './render/terrain/terrain-scene';
import type { VegetationHandle } from './render/vegetation/vegetation-scene';
import type { WorldHandle } from './render/world/world-scene';
import { M1_PERF_CAMERAS, poseYaw } from './render/perf-cameras';

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
      pause: () => void;
      renderOnce: () => void;
      setYScale: (v: number) => void;
      setDebug: (mode: string) => void;
    };
    /** `?scene=vegetation` (terrain + vegetation): vegetation diagnostics for e2e/M0-030. */
    __vegetation?: VegetationHandle;
    /** `?scene=world` (terrain + vegetation + hedgerows + buildings, M1-051b/052b): load stats and meshes. */
    __world?: WorldHandle;
    /** `?server=`: networked play (M0-022); diagnostics and e2e. */
    __netGame?: NetGameHandle;
    /** `?server=` or `?actors=demo`: placeholder actors, first-person camera and fx (M0-025); diagnostics and e2e. */
    __actors?: ActorsHandle;
    /** Placeholder (grey-box) requirement ids in use with live instance counts; feeds the missing-asset report (M1-001). */
    __placeholders?: import('./render/actors').PlaceholderEntry[];
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

  const sceneParam = params.get('scene');
  const terrainScene = sceneParam === 'terrain' || sceneParam === 'vegetation' || sceneParam === 'world'; // 'vegetation' = terrain + vegetation (M0-024); 'world' = + hedgerows + buildings (M1-051b/052b)
  // M1 camera scenes (`cam=open_field|hedgerow|old_town`): the pose also places the demo player (soldier eye view) unless `local=` is given.
  const m1cam = M1_PERF_CAMERAS.find((c) => c.name === params.get('cam'));
  if (m1cam) {
    if (!params.has('pos')) params.set('pos', m1cam.pos.join(','));
    if (!params.has('target')) params.set('target', m1cam.target.join(','));
    if (!params.has('local')) params.set('local', `${m1cam.pos[0]},${m1cam.pos[2]},${poseYaw(m1cam)}`);
  }
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
    window.__terrain = { stats: () => JSON.parse(JSON.stringify(t.stats)) as unknown, heightAt: (x, z) => t.heightAt(x, z), nodes: () => t.debugNodes(), loadMs: terrainHandle.loadMs, project: (x, z) => terrainHandle!.project(x, z), setPose: (p, tg) => terrainHandle!.setPose(p, tg), setStitch: (on) => t.setStitch(on), pause: () => terrainHandle!.pause(), renderOnce: () => terrainHandle!.renderOnce(), setYScale: (v) => t.setYScale(v), setDebug: (m) => t.setDebug(m) };
  }

  let vegHandle: VegetationHandle | null = null;
  if (terrainHandle && (sceneParam === 'vegetation' || (sceneParam === 'world' && params.get('veg') !== '0'))) {
    const { attachVegetation } = await import('./render/vegetation/vegetation-scene');
    vegHandle = await attachVegetation(ctx, terrainHandle.terrain, params);
    window.__vegetation = vegHandle;
  }

  if (terrainHandle && sceneParam === 'world') {
    const { attachWorld } = await import('./render/world/world-scene');
    window.__world = await attachWorld(ctx, terrainHandle.terrain, params);
  }

  // M0-022: networked play (`?server=host:port`): prediction, reconciliation, interpolation (client/src/game).
  let netGame: NetGameHandle | null = null;
  const bus = new EventBus<GameEvents>();
  if (params.has('server')) {
    const { startNetGame } = await import('./game/net-game');
    netGame = await startNetGame(canvas, bus);
    window.__netGame = netGame;
  }

  // M0-025: placeholder actors + first-person camera + shot feedback, on the real game state (`?server=`) or a synthetic one (`?actors=demo`).
  let actors: ActorsHandle | null = null;
  if (netGame || params.get('actors') === 'demo') {
    const { bootActors } = await import('./actors-boot');
    const ng = netGame;
    const wasm = ng ? ng.wasm : await (await import('./game/net-game')).loadWasm(import.meta.env.BASE_URL);
    actors = await bootActors({
      ctx,
      bus,
      wasm,
      params,
      state: ng?.game.state,
      localWeapon: ng ? () => ng.game.predictor.weaponDefId() : undefined,
      aiming: ng ? () => ng.sampler.isHeld('aim') : undefined,
      heightAt: terrainHandle ? (x, z) => terrainHandle!.terrain.heightAt(x, z) : undefined,
    });
    window.__actors = actors;
  }

  const overlay = new DebugOverlay(params.get('debug') === '1');
  const stepper = new FixedStepper();
  const getDump = () =>
    buildPerfDump(ctx.snapshot(), { steps_total: stepper.totalSteps, stepper_clamped: stepper.stepperClamped }, Date.now(), undefined, vegHandle?.perf());
  installPerfApi(window, getDump, () => ctx.resetStats());

  // The game update and the actors / first-person camera run before every other frame callback, so the terrain and
  // vegetation selection of this frame already see this frame's camera.
  ctx.onFrame((dt) => {
    const { steps, alpha } = stepper.advance(dt);
    netGame?.update(steps, alpha, dt);
    actors?.update(alpha, dt);
  }, 'first', 'game_update');
  if (actors && window.__perf) {
    const fx = actors.manager.fx;
    window.__perf.fireEventFrame = fx.fireEventFrames;
    window.__perf.muzzleFlashVisibleFrame = fx.muzzleFlashFrames;
  }
  ctx.onFrame(() => {
    overlay.update(performance.now(), getDump(), [...(terrainHandle?.overlayLines() ?? []), ...(vegHandle?.overlayLines() ?? []), ...(netGame?.overlayLines() ?? [])]);
  }, undefined, 'overlay');
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
