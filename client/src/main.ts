import { EventBus, FixedStepper } from './game';
import type { GameEvents } from './game';
import type { NetGameHandle } from './game/net-game';
import type { GameAudioHandle, GameAudioOutput } from './audio/game-audio';
import type { ActorsHandle } from './actors-boot';
import type { PerfApi } from './render';
import type { TerrainHandle } from './render/terrain/terrain-scene';
import type { VegetationHandle } from './render/vegetation/vegetation-scene';
import type { WorldHandle } from './render/world/world-scene';
import type { NetConfig } from './net/config';

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
async function bootWorld(
  app: HTMLElement,
  params: URLSearchParams,
  lobby?: { config: NetConfig; ticket: Uint8Array; attach: (args: MatchArgs) => () => void; audioOutput?: GameAudioOutput },
): Promise<() => void> {
  const { createRenderer, parseRendererParam, DebugOverlay, buildPerfDump, installPerfApi } = await import('./render');
  const { M1_PERF_CAMERAS, poseYaw } = await import('./render/perf-cameras');
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
    window.__taa = attachTaaScene(ctx, params.get('taa_mode') === 'pan' ? 'pan' : 'box');
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
    // dynamic casters (PI-117 follow-up): soldiers (instances of hidden source meshes), the tank, and visible tree LODs (thin instances)
    csm.addDynamicCasters((m) => {
      const n = m.name;
      if (n.startsWith('PH_V-CHR-') && m.getClassName() === 'InstancedMesh') return 'instanced';
      if (n.startsWith('PH_V-VEH-') && m.getTotalVertices() > 0) return 'plain';
      if (n.startsWith('PH_V-NAT-002__tree_') && !n.endsWith('_imp') && params.get('csm_trees') !== '0') return 'instanced';
      return null;
    }, 150);
    ctx.onFrame(() => {
      if (ctx.scene.activeCamera) csm.update(ctx.scene.activeCamera);
    }, 'first', 'csm_update');
    window.__csm = csm;
  }

  // M0-022: networked play (`?server=host:port`): prediction, reconciliation, interpolation (client/src/game).
  let netGame: NetGameHandle | null = null;
  const bus = new EventBus<GameEvents>();
  let detachUi: (() => void) | undefined;
  let actors: ActorsHandle | null = null;
  let gameAudio: GameAudioHandle | null = null;
  const disposeMatch = () => {
    detachUi?.(); detachUi = undefined;
    gameAudio?.dispose(); gameAudio = null;
    actors?.dispose(); actors = null;
  };
  const dispose = () => {
    if (netGame) netGame.dispose();
    else disposeMatch();
    ctx.dispose();
    delete window.__netGame;
    delete window.__actors;
    delete window.__perf;
    ctx.canvas.remove();
  };
  try {
    if (params.has('server') || lobby) {
      const [{ startNetGame }, { attachGameAudio }] = await Promise.all([import('./game/net-game'), import('./audio/game-audio')]);
      const ng = await startNetGame(ctx.canvas, bus, import.meta.env.BASE_URL, () => { disposeMatch(); netGame = null; }, lobby);
      netGame = ng;
      const weaponIds = new Map<number, string>();
      for (const path of Object.keys(import.meta.glob('../../data/weapons/*.json'))) {
        const id = path.slice(path.lastIndexOf('/') + 1, -'.json'.length);
        weaponIds.set(ng.wasm.weaponIndex(id), id);
      }
      const pos = ng.game.state.local.simPos;
      gameAudio = attachGameAudio(bus, {
        weaponId: (index) => weaponIds.get(index),
        listener: () => ({ x: pos[0]!, y: pos[1]!, z: pos[2]! }),
      }, lobby?.audioOutput);
      window.__netGame = ng;
    }

    // M0-025: placeholder actors + first-person camera + shot feedback, on the real game state (`?server=`) or a synthetic one (`?actors=demo`).
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
      const post = attachPost(ctx.scene, ctx.scene.activeCamera, { tier: t === 'low' || t === 'medium' ? t : 'high', taa: flag('taa'), bloom: flag('bloom'), fxaa: flag('fxaa'), tonemap: flag('tonemap'), clamp: flag('taa_clamp'), taaFactor: params.has('taa_factor') ? Number(params.get('taa_factor')) : undefined, guardMs: params.has('post_guard') ? Number(params.get('post_guard')) : undefined, velocity: params.get('taa_vel') === '1' ? { wgsl: ctx.kind === 'webgpu', terrain: terrainHandle?.terrain, flipY: params.get('taa_velflip') === '1' } : undefined });
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

    const overlay = lobby ? null : new DebugOverlay(params.get('debug') === '1');
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
      gameAudio?.flush();
    }, 'first', 'game_update');
    if (actors && window.__perf) {
      const fx = actors.manager.fx;
      window.__perf.fireEventFrame = fx.fireEventFrames;
      window.__perf.muzzleFlashVisibleFrame = fx.muzzleFlashFrames;
    }
    ctx.onFrame(() => {
      overlay?.update(performance.now(), getDump(), [...(terrainHandle?.overlayLines() ?? []), ...(vegHandle?.overlayLines() ?? []), ...(netGame?.overlayLines() ?? [])]);
    }, undefined, 'overlay');
    if (lobby && netGame) {
      await ctx.scene.whenReadyAsync();
      detachUi = lobby.attach({ canvas: ctx.canvas, game: netGame, bus, onFrame: (fn) => ctx.onFrame(fn),
        applyFov: (degrees) => {
          if (actors?.manager.rig) actors.manager.rig.setHipFovDegrees(degrees);
          else if (ctx.scene.activeCamera) ctx.scene.activeCamera.fov = degrees * Math.PI / 180;
        } });
    }
    // ?norender=1 skips the render loop (debug aid; also needed where WebGPU cannot present, e.g. SwiftShader).
    if (params.get('norender') !== '1') ctx.start();

    if (params.get('smoke') === 'compute') {
      const { runComputeSmoke } = await import('./render/compute-smoke');
      window.__smoke = await runComputeSmoke(ctx);
    }
    return dispose;
  } catch (error) {
    dispose();
    throw error;
  }
}

type MatchArgs = Omit<import('./match-ui').MatchUiOptions, 'settings' | 'persist' | 'onLeave' | 'audio'>;

async function boot(): Promise<void> {
  const params = new URLSearchParams(location.search);
  const app = document.getElementById('app');
  if (!app) throw new Error('#app missing');
  // Existing scene, smoke and direct-server URLs are developer entry points, not lobby routes.
  if (params.get('ui') !== '1' && [...params.keys()].some((key) => key !== 'uiperf')) {
    await bootWorld(app, params);
    return;
  }
  const [{ mountLobby }, { GatewayClient }, { SettingsStore }, { Persist }, { locale, setLocale }, { createAudioEngine }] = await Promise.all([
    import('./ui/lobby/boot'), import('./game/gateway-client'), import('./game/settings'), import('./input/bindings'), import('./ui/i18n'), import('./audio/engine'),
  ]);
  const persist = new Persist(() => localStorage);
  const settings = new SettingsStore(persist);
  setLocale(settings.get<'en' | 'zh-Hans'>('ui.lang'));
  locale.subscribe((lang) => settings.set('ui.lang', lang));
  let audio: import('./audio/engine').AudioApi | undefined;
  const lobbyRoot = document.createElement('div');
  app.append(lobbyRoot);
  const gateway = new GatewayClient(import.meta.env.VITE_FRONTLINE_GATEWAY_URL ?? '');
  const services: import('./ui/lobby/menu').LobbyServices = {
    gateway,
    unlockAudio() {
      try {
        audio ??= createAudioEngine({ settings });
        void audio.resume().catch(() => { /* the next gesture can retry */ });
      } catch { /* missing WebAudio must not prevent opening the menu */ }
    },
    async launch(ticket, caps, onLeave) {
      const { mountMatchUi } = await import('./match-ui');
      const root = document.createElement('div');
      app.append(root);
      const config: NetConfig = { host: ticket.host, wsPort: ticket.wsPort, wtPort: ticket.wtPort,
        secure: location.protocol === 'https:', transport: ticket.wtPort === 0 ? 'ws' : 'auto' };
      const worldParams = new URLSearchParams({ renderer: caps.backend });
      let dispose: () => void;
      try {
        dispose = await bootWorld(root, worldParams, {
          config, ticket: new TextEncoder().encode(ticket.ticket), audioOutput: audio?.gameOutput,
          attach: (args) => mountMatchUi({ ...args, settings, persist, audio, onLeave: () => {
            dispose();
            root.remove();
            audio?.stopAll();
            onLeave();
          } }),
        });
      } catch (error) {
        root.remove();
        throw error;
      }
    },
  };
  mountLobby(lobbyRoot, services);
}

boot().catch((e: unknown) => {
  console.error('[boot] failed', e);
  const app = document.getElementById('app');
  if (app) app.textContent = `Boot failed: ${e instanceof Error ? e.message : String(e)}`;
});
