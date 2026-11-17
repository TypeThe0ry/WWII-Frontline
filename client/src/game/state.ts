/**
 * Read-only view of the game for render / ui / audio (they never import `game` internals: they read this
 * and listen to the event bus). Hot data (poses) lives in typed arrays updated in place every frame;
 * counters and values the overlay shows are `@preact/signals` updated once per snapshot.
 */
import { signal, type Signal } from '@preact/signals';
import { MAX_ENTITIES } from './interpolation';

export interface LocalView {
  /** Render position (interpolated between the last two prediction steps + smoothing offset). */
  pos: Float32Array; // x y z
  /** Position after the latest prediction step. */
  simPos: Float32Array;
  vel: Float32Array;
  yaw: number;
  pitch: number;
  /** 0 stand, 1 crouch. */
  stance: number;
  grounded: boolean;
  /** Predicted ammo (display), authoritative values in `ammoAuth`. */
  ammo: number;
  reserve: number;
  recoilPitch: number;
  recoilYaw: number;
}

export interface EntityView {
  /** Number of valid slots. */
  count: number;
  ids: Uint8Array;
  /** x y z per slot (render-interpolated). */
  pos: Float32Array;
  yaw: Float32Array;
  pitch: Float32Array;
  anim: Uint8Array;
  stance: Uint8Array;
  weapon: Uint8Array;
  flags: Uint8Array;
  health: Uint8Array;
}

export interface NetView {
  corrections: Signal<number>;
  snaps: Signal<number>;
  lastErrM: Signal<number>;
  rttMs: Signal<number>;
  /** Authoritative values from the last snapshot's `self` block. */
  ammoAuth: Signal<number>;
  reserveAuth: Signal<number>;
  health: Signal<number>;
  stamina: Signal<number>;
  snapshotTick: Signal<number>;
  /** Upstream input rate over the last second (kbit/s, payload only). */
  upKbps: Signal<number>;
}

export class GameState {
  readonly local: LocalView = {
    pos: new Float32Array(3),
    simPos: new Float32Array(3),
    vel: new Float32Array(3),
    yaw: 0,
    pitch: 0,
    stance: 0,
    grounded: false,
    ammo: 0,
    reserve: 0,
    recoilPitch: 0,
    recoilYaw: 0,
  };
  readonly entities: EntityView = {
    count: 0,
    ids: new Uint8Array(MAX_ENTITIES),
    pos: new Float32Array(MAX_ENTITIES * 3),
    yaw: new Float32Array(MAX_ENTITIES),
    pitch: new Float32Array(MAX_ENTITIES),
    anim: new Uint8Array(MAX_ENTITIES),
    stance: new Uint8Array(MAX_ENTITIES),
    weapon: new Uint8Array(MAX_ENTITIES),
    flags: new Uint8Array(MAX_ENTITIES),
    health: new Uint8Array(MAX_ENTITIES),
  };
  readonly net: NetView = {
    corrections: signal(0),
    snaps: signal(0),
    lastErrM: signal(0),
    rttMs: signal(0),
    ammoAuth: signal(0),
    reserveAuth: signal(0),
    health: signal(0),
    stamina: signal(0),
    snapshotTick: signal(0),
    upKbps: signal(0),
  };
}
