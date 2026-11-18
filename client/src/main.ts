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
    /** `?scene=pbr`: PBR material test scene and prewarm result (M1-042b). */
    __pbr?: import('./render/materials/pbr-scene').PbrSceneHandle;
    /** `?post=high|medium|low`: post chain (M1-044b). */
    __post?: import('./render/post/post-chain').PostHandle;
    /** `?csm=custom`: custom cascaded shadow maps sampled by terrain/PBR shaders (PI-117). */
    __csm?: import('./render/lighting/custom-csm').CustomCsm;
    /** `?scene=taa`: deterministic ghosting test scene (M1-044b). */
    __taa?: import('./render/post/taa-scene').TaaSceneHandle;
    /** `?vehicle=demo`: placeholder tank (M1-048b). */
    __vehicle?: import('./render/vehicles/vehicle-demo').VehicleDemoHandle;
    /** `?vfx=stress`: particle stress hook (M1-047b). */
    __vfx?: import('./render/vfx/vfx-scene').VfxHandle;
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
    empty: params.get('empty') === '1' || terrainScene || sceneParam === 'pbr' || sceneParam === 'taa',
  });
  if (sceneParam === 'taa') {
    const { attachTaaScene } = await import('./render/post/taa-scene');
    window.__taa = attachTaaScene(ctx);
  }
  if (sceneParam === 'pbr') {
    const { attachPbrScene } = await import('./render/materials/pbr-scene');
    window.__pbr = await attachPbrScene(ctx, params);
  }
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

  if (params.get('vfx') === 'stress') {
    const { attachVfxStress } = await import('./render/vfx/vfx-scene');
    window.__vfx = attachVfxStress(ctx, params);
  }

  if (terrainHandle && window.__world && params.get('csm') === 'custom') {
    const { attachCustomCsm } = await import('./render/lighting/custom-csm');
    const tier = (params.get('csm_tier') ?? 'high') as import('./render/lighting/csm-math').ShadowTier;
    const csm = attachCustomCsm(ctx, { tier, lightDir: [0.45, -0.8, 0.35], webgl2Config: params.get('csm_cfg') === 'webgl2' || undefined, flipY: params.has('csm_flipy') ? params.get('csm_flipy') === '1' : undefined, halfZ: params.has('csm_halfz') ? params.get('csm_halfz') === '1' : undefined });
    for (const m of window.__world.buildingMeshes) csm.addCaster(m);
    if (window.__world.hedges) csm.addCaster(window.__world.hedges, true);
    csm.attach(terrainHandle.terrain.material);
    ctx.onFrame(() => {
      if (ctx.scene.activeCamera) csm.update(ctx.scene.activeCamera);
    }, 'first', 'csm_update');
    window.__csm = csm;
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

  if (params.get('vehicle') === 'demo') {
    const { attachVehicleDemo } = await import('./render/vehicles/vehicle-demo');
    const { PlaceholderRegistry } = await import('./render/actors/names');
    const reg = actors ? actors.manager.registry : new PlaceholderRegistry();
    window.__vehicle = await attachVehicleDemo(ctx, params, actors?.manager.fx ?? null, terrainHandle ? (x, z) => terrainHandle!.terrain.heightAt(x, z) : () => 0, reg);
    if (!actors) reg.install(window);
  }

  if (params.has('post') && ctx.scene.activeCamera) {
    const { attachPost } = await import('./render/post/post-chain');
    const t = params.get('post');
    const flag = (k: string): boolean | undefined => (params.has(k) ? params.get(k) === '1' : undefined);
    const post = attachPost(ctx.scene, ctx.scene.activeCamera, { tier: t === 'low' || t === 'medium' ? t : 'high', taa: flag('taa'), bloom: flag('bloom'), fxaa: flag('fxaa'), tonemap: flag('tonemap'), clamp: flag('taa_clamp'), taaFactor: params.has('taa_factor') ? Number(params.get('taa_factor')) : undefined, guardMs: params.has('post_guard') ? Number(params.get('post_guard')) : undefined });
    window.__post = post;
    ctx.onFrame((dtMs) => post.update(dtMs / 1000), 'first', 'post_update');
  }

  // M1-046b: first-person weapon wall pushback against the building meshes of the world scene
  if (actors?.manager.rig && window.__world) {
    const { Ray } = await import('@babylonjs/core/Culling/ray');
    const { Vector3 } = await import('@babylonjs/core/Maths/math.vector');
    const o = new Vector3();
    const d = new Vector3();
    const ray = new Ray(o, d, 1);
    actors.manager.rig.wallProbe = (ox, oy, oz, dx, dy, dz, max) => {
      o.set(ox, oy, oz);
      d.set(dx, dy, dz);
      ray.length = max;
      const hit = ctx.scene.pickWithRay(ray, (m) => m.metadata?.blocks === true);
      return hit?.hit ? hit.distance : null;
    };
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
