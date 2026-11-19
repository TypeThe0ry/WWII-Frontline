/**
 * Input sampling (ARCHITECTURE 6.3): key / mouse events are collected per render frame, mouse deltas
 * accumulate until the next simulation step, and `build()` turns the state into one `InputCmd` per
 * step. Yaw / pitch are owned by the client (the server follows the commands).
 */
import { auxFor, buttonsFor, DEFAULT_KEYMAP, moveAxes, type Action, type MoveAxes } from './keymap';

export const TAU = Math.PI * 2;
/** Initial mouse sensitivity in degrees per pixel of `movementX/Y` (a pure input preference; localStorage `frontline.sens`). */
export const DEFAULT_SENSITIVITY_DEG_PER_PX = 0.1;
const HALF_PI = Math.PI / 2;

export interface InputCmdFields {
  seq: number;
  tick: number;
  moveX: number;
  moveZ: number;
  /** u16: 0..65535 -> [0, 2 pi). */
  yaw: number;
  /** i16: -32768..32767 -> [-pi/2, +pi/2]. */
  pitch: number;
  buttons: number;
  weaponSlot: number;
  aux: number;
  viewTick: number;
}

/** Anything that can produce a command for one simulation step (the real sampler, or a test script). */
export interface InputSource {
  build(seq: number, tick: number, viewTick: number): InputCmdFields;
  /** The window lost focus / the tab went to the background / pointer lock was lost: all keys up. */
  releaseAll(): void;
}

export const quantYaw = (rad: number): number => {
  const t = ((rad % TAU) + TAU) % TAU;
  return Math.round((t / TAU) * 65536) & 0xffff;
};
export const quantPitch = (rad: number): number => {
  const c = Math.max(-HALF_PI, Math.min(HALF_PI, rad));
  // inverse of protocol `pitch_i16_to_rad`: (p + 0.5) / 32767.5 * pi/2
  return Math.max(-32768, Math.min(32767, Math.round((c / HALF_PI) * 32767.5 - 0.5)));
};
export const yawFromCode = (code: number): number => ((code & 0xffff) / 65536) * TAU;
export const pitchFromCode = (code: number): number => ((code + 0.5) / 32767.5) * HALF_PI;

export class InputSampler implements InputSource {
  private readonly held = new Set<Action>();
  private readonly down = new Set<string>();
  private pressed?: (code: string) => void;
  private cleared?: () => void;
  private dx = 0;
  private dy = 0;
  private readonly axes: MoveAxes = { moveX: 0, moveZ: 0 };
  yaw = 0;
  pitch = 0;
  /** Degrees per pixel; horizontal and vertical share it in M0. */
  sensitivity = DEFAULT_SENSITIVITY_DEG_PER_PX;
  invertY = false;
  weaponSlot = 0;

  constructor(private readonly keymap: Readonly<Record<string, Action>> = DEFAULT_KEYMAP) {}

  /** `code` is `KeyboardEvent.code` or `Mouse<button>`. Repeats are idempotent. */
  press(code: string): void {
    if (!this.down.has(code)) this.pressed?.(code);
    this.down.add(code);
    const a = this.keymap[code];
    if (a) this.held.add(a);
  }

  release(code: string): void {
    this.down.delete(code);
    const a = this.keymap[code];
    if (!a) return;
    // Another physical key may still hold the same action.
    for (const d of this.down) if (this.keymap[d] === a) return;
    this.held.delete(a);
  }

  /** Raw mouse movement in pixels; accumulated until the next `build`. */
  mouseMove(dx: number, dy: number): void {
    if (Number.isFinite(dx)) this.dx += dx;
    if (Number.isFinite(dy)) this.dy += dy;
  }

  releaseAll(): void {
    this.down.clear();
    this.held.clear();
    this.dx = 0;
    this.dy = 0;
    this.cleared?.();
  }

  /** Optional match input context; receives edges even between two fixed ticks. */
  onInputContext(press: (code: string) => void, clear: () => void): void {
    this.pressed = press;
    this.cleared = clear;
  }

  isHeld(a: Action): boolean {
    return this.held.has(a);
  }

  setView(yawRad: number, pitchRad: number): void {
    this.yaw = ((yawRad % TAU) + TAU) % TAU;
    this.pitch = Math.max(-HALF_PI, Math.min(HALF_PI, pitchRad));
  }

  build(seq: number, tick: number, viewTick: number): InputCmdFields {
    const k = (this.sensitivity * Math.PI) / 180;
    this.yaw = (((this.yaw + this.dx * k) % TAU) + TAU) % TAU;
    this.pitch = Math.max(-HALF_PI, Math.min(HALF_PI, this.pitch - this.dy * k * (this.invertY ? -1 : 1)));
    this.dx = 0;
    this.dy = 0;
    moveAxes(this.held, this.axes);
    return {
      seq: seq & 0xffff,
      tick,
      moveX: this.axes.moveX,
      moveZ: this.axes.moveZ,
      yaw: quantYaw(this.yaw),
      pitch: quantPitch(this.pitch),
      buttons: buttonsFor(this.held),
      weaponSlot: this.weaponSlot,
      aux: auxFor(this.held),
      viewTick: Math.max(0, Math.min(0xffff, viewTick)),
    };
  }
}
