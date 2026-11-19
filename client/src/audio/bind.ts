/**
 * Connects the game event bus to the `AudioDirector` (M1-063b): remote and local shots, footsteps and suppression
 * become audio events. Pure wiring (no WebAudio): the caller supplies the clock, the weapon-name lookup and the
 * listener position, and calls `director.flush(now)` once per tick for the footsteps.
 */
import type { EventBus, GameEvents } from '../game/event-bus';
import type { Surface } from './crossfade';
import type { AudioDirector } from './dispatch';
import type { Vec3 } from './voices';

export interface AudioBindDeps {
  /** Audio clock, seconds. */
  now: () => number;
  /** Data id of a wire weapon index (e.g. `us_m1_garand`); undefined = unknown (no sound). */
  weaponId: (index: number) => string | undefined;
  /** Position of the local player (muzzle of `fire.local` is the listener's own, so only used for the listener). */
  listener: () => Vec3;
  /** Ground material under a footstep; default grass. */
  surfaceAt?: (pos: Vec3) => Surface;
  /** Drop events instead of queueing them against a suspended realtime audio clock. */
  canPlay?: () => boolean;
  /** Apply the director's new filter immediately, on the event's audio clock. */
  onSuppress?: (atS: number) => void;
}

/** Returns the unsubscribe function. */
export function bindAudioEvents(bus: EventBus<GameEvents>, dir: AudioDirector, d: AudioBindDeps): () => void {
  const offs = [
    bus.on('fire.remote', (e) => {
      if (d.canPlay?.() === false) return;
      const w = d.weaponId(e.weapon);
      if (w === undefined) return;
      dir.listener = d.listener();
      dir.handle({ kind: 'shot', weapon: w, pos: { x: e.muzzlePos.x, y: e.muzzlePos.y, z: e.muzzlePos.z } }, d.now());
    }),
    bus.on('fire.local', (e) => {
      if (d.canPlay?.() === false) return;
      const w = d.weaponId(e.weapon);
      if (w === undefined) return;
      dir.listener = d.listener();
      dir.handle({ kind: 'shot', weapon: w, pos: { x: e.muzzlePos.x, y: e.muzzlePos.y, z: e.muzzlePos.z } }, d.now());
    }),
    bus.on('entity.footstep', (e) => {
      if (d.canPlay?.() === false) return;
      dir.listener = d.listener();
      const pos = { x: e.pos.x, y: e.pos.y, z: e.pos.z };
      dir.handle({ kind: 'footstep', pos, gait: e.stance === 0 ? 'walk' : 'crouch', surface: d.surfaceAt?.(pos) ?? 'grass', actor: e.entity }, d.now());
    }),
    bus.on('suppress', (e) => {
      if (d.canPlay?.() === false) return;
      const at = d.now();
      dir.handle({ kind: 'suppress', intensity: e.intensity * 255 }, at);
      d.onSuppress?.(at);
    }),
  ];
  return () => offs.forEach((off) => off());
}
