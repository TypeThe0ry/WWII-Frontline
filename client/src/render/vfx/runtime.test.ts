import { VertexBuffer } from '@babylonjs/core/Buffers/buffer';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { CreateBox } from '@babylonjs/core/Meshes/Builders/boxBuilder';
import { TransformNode } from '@babylonjs/core/Meshes/transformNode';
import { afterEach, describe, expect, it } from 'vitest';
import { EventBus, type GameEvents } from '../../game/event-bus';
import { EV_EXPLOSION, EV_IMPACT, EventDecoder } from '../../game/events';
import { LAYOUT_EVENT_RECORD_BYTES } from '../../net/wasm';
import { PlaceholderRegistry } from '../actors/names';
import { makeScene, type TestScene } from '../actors/test-support';
import { FxManager } from '../fx';
import { LAYER } from '../terrain/terrain-splat';
import { DECAL_DISTANCE_M, decodeImpactNormal } from './decals';
import { KIND_ID, type VfxTier } from './particle-pool';

const UP = (32 << 6) | 32;
const INTO_CAMERA = 32 << 6; // near -Z (server oct encoding)
const SHOT = { weapon: 0, muzzlePos: { x: 0, y: 2, z: 1 }, dir: { x: 0, y: 0, z: 1 } };
let scene: TestScene | undefined;
let fx: FxManager | undefined;

afterEach(() => {
  fx?.dispose();
  fx = undefined;
  scene?.dispose();
  scene = undefined;
});

function setup(tier: VfxTier = 'medium') {
  const ts = scene = makeScene();
  const bus = new EventBus<GameEvents>();
  const local = new TransformNode('local_muzzle', ts.scene);
  const remote = new TransformNode('remote_muzzle', ts.scene);
  local.position.set(1, 2, 1);
  remote.position.set(3, 2, 1);
  let now = 0;
  const effects = fx = new FxManager({
    scene: ts.scene, bus, registry: new PlaceholderRegistry(), weapons: new Map(),
    localMuzzle: () => local, remoteMuzzle: (id) => id === 7 ? remote : null,
    particles: { tier, wgsl: false }, now: () => now,
  });
  const live = () => {
    const pool = effects.particlePool!;
    const output = new Float32Array(pool.capacity * 16);
    const n = pool.writeInstances(output);
    return Array.from({ length: n }, (_, i) => ({
      kind: output[i * 16 + 1], age: output[i * 16 + 2],
      position: Array.from(output.subarray(i * 16 + 12, i * 16 + 15)),
    }));
  };
  return { ts, bus, effects, live, advance: (ms: number) => { now += ms; } };
}

function record(id: number, type: number, pos: readonly [number, number, number], suffix: number[]): Uint8Array {
  const bytes = new Uint8Array(LAYOUT_EVENT_RECORD_BYTES);
  const view = new DataView(bytes.buffer);
  view.setUint16(0, id, true);
  bytes[2] = type;
  for (let i = 0; i < pos.length; i++) view.setUint16(3 + i * 2, pos[i]! * 64, true);
  bytes.set(suffix, 9);
  return bytes;
}

describe('M1 VFX render lifecycle', () => {
  it('uploads both muzzle sockets and fallback positions at age zero after a slow frame', () => {
    const { ts, bus, effects, live, advance } = setup();
    ts.frame(() => {
      advance(200);
      bus.emit('fire.local', SHOT);
      bus.emit('fire.remote', { ...SHOT, shooter: 7, distance: 6 });
      bus.emit('fire.remote', { ...SHOT, shooter: 8, distance: 6 });
    });
    const flashes = live().filter((p) => p.kind === KIND_ID.muzzle_flash);
    expect(flashes.map((p) => p.position)).toEqual([[1, 2, 1], [3, 2, 1], [0, 2, 1]]);
    expect(flashes.map((p) => p.age)).toEqual([0, 0, 0]);
    expect(effects.muzzleFlashFrames).toEqual(effects.fireEventFrames);
    expect(effects.particles!.drawn).toBe(4);
    ts.frame(() => advance(60));
    expect(live().map((p) => p.kind)).toEqual([KIND_ID.shell]);
  });

  it('renders decoded explosion and material impact events in the next pass, with no legacy dust', () => {
    const { ts, bus, effects, live } = setup();
    effects.setImpactSurfaces([], () => 0);
    const decoder = new EventDecoder(bus, () => ts.camera.globalPosition);
    const impact = record(1, EV_IMPACT, [2, 0, 4], [UP & 255, UP >> 8, 4]);
    const explosion = record(2, EV_EXPLOSION, [5, 0, 8], [0, 64]);
    ts.frame(() => {
      decoder.decode(impact);
      decoder.decode(explosion); // decoder reuses the position object
      decoder.decode(impact); // redundant record must not produce another effect
      expect(effects.particles!.drawn).toBe(0);
    });
    expect(effects.decals!.marks.total).toBe(1);
    expect(Array.from(effects.decals!.marks.pos.subarray(0, 3))).toEqual([2, 0, 4]);
    expect(effects.decals!.marks.material[0]).toBe(4);
    expect(effects.decals!.mesh.isEnabled()).toBe(true);
    expect(effects.particles!.drawn).toBe(11);
    expect(live().map((p) => p.kind)).toEqual([4, 4, 3, 3, 3, 2, 2, 2, 2, 2, 2]);
    expect(effects.dust.activeDust).toBe(0);
    expect(effects.dust.activeMarks).toBe(0);
  });

  it('keeps muzzle and fifth-round tracer at Low and skips shells and near dust', () => {
    const { ts, bus, effects, live } = setup('low');
    ts.frame(() => {
      for (let i = 0; i < 4; i++) {
        bus.emit('fire.local', SHOT);
        bus.emit('fire.remote', { ...SHOT, shooter: 7, distance: 6 });
      }
      bus.emit('impact', { pos: { x: 0, y: 0, z: 1 }, normal: UP, material: 0 });
    });
    expect(effects.tracers.active).toBe(0);
    expect(live().map((p) => p.kind)).toEqual(new Array(8).fill(KIND_ID.muzzle_flash));
    ts.frame(() => {
      bus.emit('fire.local', SHOT);
      bus.emit('impact', { pos: { x: 0, y: 0, z: 110 }, normal: UP, material: 0 });
    });
    expect(effects.tracers.active).toBe(1);
    expect(live().at(-1)).toMatchObject({ kind: KIND_ID.muzzle_flash, position: [0, 0, 110] });
  });

  it('uses the dominant terrain layer for dust and decal style on heightfield hits', () => {
    const { ts, bus, effects, live } = setup();
    effects.setImpactSurfaces([], () => 0, (x) => x < 2 ? LAYER.sand : LAYER.rock);
    ts.frame(() => {
      bus.emit('impact', { pos: { x: 1, y: 0, z: 2 }, normal: UP, material: 4 });
      bus.emit('impact', { pos: { x: 3, y: 0, z: 2 }, normal: UP, material: 4 });
    });
    expect(live()).toHaveLength(5 + 3); // sand and stone, instead of two metal bursts
    expect(effects.decals!.marks.active).toBe(2);
    const colors = effects.decals!.mesh.getVerticesData(VertexBuffer.ColorKind)!;
    expect(colors[4]).toBeCloseTo(0.38);
    expect(colors[18 * 4]).toBeCloseTo(0.34);
  });

  it('keeps a fresh tracer at the muzzle on a slow frame, then bends and expires it', () => {
    const { ts, bus, effects, advance } = setup();
    ts.frame(() => {
      advance(250);
      for (let i = 0; i < 5; i++) bus.emit('fire.local', SHOT);
    });
    const tracer = ts.scene.meshes.find((mesh) => mesh.name.startsWith('PH_V-FX-006__t') && mesh.isEnabled())!;
    expect(tracer.position.z).toBeCloseTo(2.25);
    expect(tracer.position.y).toBeCloseTo(2);
    ts.frame(() => advance(200));
    expect(tracer.position.z).toBeCloseTo(122.25);
    expect(tracer.position.y).toBeLessThan(1.81);
    ts.frame(() => advance(101));
    expect(effects.tracers.active).toBe(0);
  });

  it('queues a tank shot and disposes its scene resources and event subscriptions', () => {
    const { ts, bus, effects } = setup();
    effects.setImpactSurfaces([]);
    const decals = effects.decals!;
    const particles = effects.particles!;
    ts.frame(() => effects.emitTankShot(0, 2, 1));
    expect(particles.drawn).toBe(5);
    effects.dispose();
    fx = undefined;
    expect(decals.mesh.isDisposed()).toBe(true);
    expect(particles.mesh.isDisposed()).toBe(true);
    ts.frame(() => bus.emit('fire.local', SHOT));
    expect(effects.fireEventFrames).toHaveLength(0);
  });
});

describe('procedural impact decals', () => {
  it('decodes the C2 octahedral axes and the folded lower hemisphere', () => {
    expect(decodeImpactNormal(UP, new Vector3()).y).toBeGreaterThan(0.999);
    expect(decodeImpactNormal(INTO_CAMERA, new Vector3()).z).toBeLessThan(-0.999);
    expect(decodeImpactNormal(0, new Vector3()).y).toBe(-1);
  });

  it('reprojects onto registered walls and ignores distant, unregistered and failed probes', () => {
    const { ts, bus, effects } = setup();
    const wall = CreateBox('static_wall', { width: 4, height: 4, depth: 0.5 }, ts.scene);
    wall.position.set(0, 2, 5);
    const dynamic = CreateBox('unregistered_actor', { size: 1 }, ts.scene);
    dynamic.position.set(5, 2, 5);
    effects.setImpactSurfaces([wall], () => 0);
    const impact = (x: number, y: number, z: number, normal: number) => bus.emit('impact', { pos: { x, y, z }, normal, material: 2 });
    ts.frame(() => {
      impact(0, 2, 4.77, INTO_CAMERA); // slightly inside the wall, as can happen with quantization
      impact(5, 2, 4.5, INTO_CAMERA);
      impact(4, 2, 4.5, INTO_CAMERA);
      impact(0, 0, DECAL_DISTANCE_M + 1, UP);
    });
    const decals = effects.decals!;
    expect(decals.marks.total).toBe(1);
    expect(decals.marks.pos[2]).toBeCloseTo(4.75, 5);
    const p = decals.mesh.getVerticesData(VertexBuffer.PositionKind)!;
    expect(p[2]).toBeCloseTo(4.747, 5);
    expect(p[5]).toBeCloseTo(p[2]!, 5); // disk is parallel to wall
    const alpha = () => decals.mesh.getVerticesData(VertexBuffer.ColorKind)![3]!;
    expect(alpha()).toBeCloseTo(0.91);
    ts.camera.position.z = 50;
    ts.frame();
    expect(alpha()).toBeLessThan(0.91);
    expect(alpha()).toBeGreaterThan(0);
    ts.camera.position.z = 80;
    ts.frame();
    expect(decals.mesh.isEnabled()).toBe(false);
  });

  it('recycles the rendered ring after 500 placements and clears it on receiver replacement', () => {
    const { ts, bus, effects } = setup();
    effects.setImpactSurfaces([], () => 0);
    ts.frame(() => {
      for (let i = 0; i < 500; i++) bus.emit('impact', { pos: { x: i / 16, y: 0, z: 2 }, normal: UP, material: 0 });
    });
    const decals = effects.decals!;
    expect(decals.marks.active).toBe(128);
    expect(decals.marks.total).toBe(500);
    expect(decals.marks.pos[115 * 3]).toBe(499 / 16);
    expect(decals.marks.pos[116 * 3]).toBe(372 / 16);
    effects.setImpactSurfaces([], () => 0);
    ts.frame(() => bus.emit('impact', { pos: { x: 2, y: 0, z: 2 }, normal: UP, material: 0 }));
    expect(decals.marks.active).toBe(1);
    const colors = decals.mesh.getVerticesData(VertexBuffer.ColorKind)!;
    expect(colors[3]).toBeCloseTo(0.91);
    expect(colors[17 * 4 + 3]).toBe(0);
  });
});
