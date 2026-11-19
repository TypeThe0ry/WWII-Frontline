import type { ArcRotateCamera } from '@babylonjs/core/Cameras/arcRotateCamera';
import { EventBus, type GameEvents } from '../src/game/event-bus';
import { GameState } from '../src/game/state';
import { ActorManager, type ActorData } from '../src/render/actors';
import { MOTION } from '../src/render/actors/anim-graph';
import { createRenderer, parseRendererParam } from '../src/render/engine';

// No game server, map, WASM, clips, or model assets. Test drives the same SoA and bus the renderer reads in-game.
const data: ActorData = {
  capsule: { radius: 0.24, heightStand: 1.8, heightCrouch: 1.2, eyeStand: 1.65, eyeCrouch: 1.1 },
  stanceTransitionMs: 250,
  hipFovDeg: 80,
  weapons: [{
    index: 0, id: 'fixture', requirementId: 'V-WPN-US-001', faction: 'us', muzzleFlashMs: 50, tracer: false,
    muzzleVelocityMps: 600, adsMs: 200,
    dims: { length: 1, barrelLength: 0.5, barrelDiameter: 0.04, receiverHeight: 0.09, receiverWidth: 0.05, magLength: 0.08, magHeight: 0.07, magWidth: 0.04 },
    view: { hipOffset: [0, 0, 0], adsOffset: [0, 0, 0], adsFovMult: 1.2 },
  }],
};

async function setup() {
  const canvas = document.querySelector('canvas')!;
  const renderer = parseRendererParam(new URLSearchParams(location.search).get('renderer'));
  const ctx = await createRenderer(canvas, { renderer, empty: true, fly: false });
  const camera = ctx.scene.activeCamera as ArcRotateCamera;
  camera.radius = 26;
  camera.target.set(0, 1, 8);
  camera.detachControl();
  const bus = new EventBus<GameEvents>();
  const state = new GameState();
  const e = state.entities;
  e.count = 60;
  for (let k = 0; k < e.count; k++) {
    e.ids[k] = 10 + k;
    e.pos[k * 3] = ((k % 10) - 4.5) * 1.8;
    e.pos[k * 3 + 2] = Math.floor(k / 10) * 4;
    e.yaw[k] = Math.PI * 0.75;
    e.health[k] = 255;
    e.flags[k] = k % 2 ? 0x80 : 0;
    e.stance[k] = k < 2 ? 0 : k % 3;
    e.anim[k] = k < 2 ? MOTION.idle : (k % 3) + 1;
  }
  let now = 0;
  const manager = new ActorManager({ scene: ctx.scene, state, bus, data, now: () => now, tracers: false });
  const frame = (dtMs: number): void => {
    now += dtMs;
    ctx.engine.beginFrame();
    manager.update(1, dtMs);
    ctx.scene.render();
    ctx.engine.endFrame();
  };
  frame(0);
  await ctx.scene.whenReadyAsync();
  frame(0);
  return { backend: ctx.kind, state, bus, manager, frame, stats: () => ctx.snapshot() };
}

declare global {
  interface Window {
    actorFixture?: Awaited<ReturnType<typeof setup>>;
  }
}

window.actorFixture = await setup();
