/**
 * Synthetic game state for `?actors=demo` (no server): N remote soldiers (a block ahead of the local player, each walking a
 * small circle at constant speed); the local player stands still. Lets e2e tests, screenshots and the M0-030 baselines exercise the actor path without
 * a server. Implements the same `ActorFeed` the real `GameState` provides.
 */
import type { ActorFeed } from './actor-manager';

export interface DemoFeedOptions {
  count: number;
  /** Local player feet position and yaw. */
  local: { x: number; z: number; yaw: number };
  heightAt(x: number, z: number): number;
  /** Weapon index of the two factions' rifles (alternating). */
  weapons: [number, number];
  speedMps?: number;
}

export interface DemoFeed {
  state: ActorFeed;
  /** Advances the entities by `dtMs` and rewrites the entity arrays. */
  step(dtMs: number): void;
}

export function createDemoFeed(o: DemoFeedOptions): DemoFeed {
  const n = o.count;
  const speed = o.speedMps ?? 3;
  const state: ActorFeed = {
    local: {
      pos: new Float32Array([o.local.x, o.heightAt(o.local.x, o.local.z), o.local.z]),
      simPos: new Float32Array(3),
      vel: new Float32Array(3),
      yaw: o.local.yaw,
      pitch: 0,
      stance: 0,
      grounded: true,
      ammo: 8,
      reserve: 80,
      recoilPitch: 0,
      recoilYaw: 0,
    },
    entities: {
      count: n,
      ids: new Uint8Array(n),
      pos: new Float32Array(n * 3),
      yaw: new Float32Array(n),
      pitch: new Float32Array(n),
      anim: new Uint8Array(n),
      stance: new Uint8Array(n),
      weapon: new Uint8Array(n),
      flags: new Uint8Array(n),
      health: new Uint8Array(n).fill(255),
    },
  };
  let t = 0;
  const e = state.entities;
  // A block of soldiers ahead of the local player (grid, 2.5 m apart); each one walks a small circle at constant speed.
  const cols = Math.max(1, Math.ceil(Math.sqrt(n)));
  const place = (): void => {
    for (let k = 0; k < n; k++) {
      const gx = o.local.x + ((k % cols) - (cols - 1) / 2) * 2.5;
      const gz = o.local.z + 12 + Math.floor(k / cols) * 2.5;
      const rad = 0.8;
      const dir = k % 2 === 0 ? 1 : -1;
      const a = k * 2.399 + (t * dir * speed) / rad;
      const x = gx + Math.cos(a) * rad;
      const z = gz + Math.sin(a) * rad;
      e.ids[k] = (k + 10) & 255;
      e.pos[k * 3] = x;
      e.pos[k * 3 + 1] = o.heightAt(x, z);
      e.pos[k * 3 + 2] = z;
      e.yaw[k] = Math.atan2(-Math.sin(a) * dir, Math.cos(a) * dir);
      e.stance[k] = k % 7 === 3 ? 1 : 0;
      e.weapon[k] = o.weapons[k % 2] as number;
      e.flags[k] = k % 2 === 0 ? 0 : 0x80; // bit 7 = faction (PI-024)
    }
  };
  place();
  return {
    state,
    step(dtMs) {
      t += dtMs / 1000;
      place();
    },
  };
}
