/**
 * Composition of the actor path (M0-025): data -> `ActorManager` (+ first-person camera, fx) -> DOM overlays.
 * Used by `main.ts` for `?server=...` (real game state) and `?actors=demo` (synthetic state, no server).
 */
import { ActorManager, loadActorData, type ActorFeed } from './render/actors';
import { createDemoFeed, type DemoFeed } from './render/actors/demo-feed';
import type { EventBus, GameEvents } from './game/event-bus';
import type { RenderContext } from './render/engine';
import type { SimWasmModule } from './net/wasm';
import { createCrosshair, createHitMarker } from './ui';
import { UniversalCamera } from '@babylonjs/core/Cameras/universalCamera';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';

export interface ActorsHandle {
  manager: ActorManager;
  bus: EventBus<GameEvents>;
  feed: DemoFeed | null;
  state: ActorFeed;
  /** Once per render frame, after the game update. */
  update(alpha: number, dtMs: number): void;
  dispose(): void;
}

export interface ActorsBootOptions {
  ctx: RenderContext;
  bus: EventBus<GameEvents>;
  wasm: SimWasmModule;
  params: URLSearchParams;
  /** Real game state (`?server=`); omitted for the demo feed. */
  state?: ActorFeed;
  localWeapon?: () => number;
  aiming?: () => boolean;
  heightAt?: (x: number, z: number) => number;
}

/** The scene's free camera, or a new one when the scene has an orbit camera (the plain test scene). */
function firstPersonCamera(ctx: RenderContext): UniversalCamera {
  const cur = ctx.scene.activeCamera;
  if (cur instanceof UniversalCamera) {
    cur.maxZ = Math.max(cur.maxZ, 12000);
    return cur;
  }
  const cam = new UniversalCamera('fp_camera', new Vector3(0, 2, 0), ctx.scene);
  cam.minZ = 0.1;
  cam.maxZ = 12000;
  cur?.detachControl();
  ctx.scene.activeCamera = cam;
  return cam;
}

export async function bootActors(o: ActorsBootOptions): Promise<ActorsHandle> {
  const { ctx, bus, wasm, params } = o;
  const data = await loadActorData(wasm, import.meta.env.BASE_URL);
  let feed: DemoFeed | null = null;
  let state = o.state;
  let localWeapon = o.localWeapon;
  let aiming = o.aiming;
  if (!state) {
    const w = data.weapons;
    const us = w.find((x) => x.faction === 'us') ?? (w[0] as (typeof w)[number]);
    const ger = w.find((x) => x.faction === 'ger') ?? us;
    const n = Math.max(0, Math.min(200, Number(params.get('entities') ?? 10) | 0));
    feed = createDemoFeed({ count: n, local: { x: 128, z: 112, yaw: 0 }, heightAt: o.heightAt ?? (() => 0), weapons: [us.index, ger.index] });
    state = feed.state;
    localWeapon = () => us.index;
    aiming = () => params.get('ads') === '1';
  }
  const manager = new ActorManager({
    scene: ctx.scene,
    bus,
    state,
    data,
    camera: firstPersonCamera(ctx),
    localWeapon,
    aiming,
    tracers: params.get('tracers') !== '0',
  });
  manager.registry.install(window);
  // e2e hook: names of every mesh / material in the scene (naming rule check)
  (window as unknown as { __actorsNames?: () => string[] }).__actorsNames = () => [...ctx.scene.meshes.map((m) => m.name), ...ctx.scene.materials.map((m) => m.name)];
  const overlay = document.body;
  const crosshair = createCrosshair(overlay, bus);
  const hit = createHitMarker(overlay, bus);
  return {
    manager,
    bus,
    feed,
    state,
    update(alpha, dtMs) {
      feed?.step(dtMs);
      manager.update(alpha, dtMs);
    },
    dispose() {
      crosshair.dispose();
      hit.dispose();
      manager.dispose();
    },
  };
}
