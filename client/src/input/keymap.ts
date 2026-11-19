/**
 * Default key bindings (constants; rebinding is M2-041). `KeyboardEvent.code` based, so they do not
 * depend on the keyboard layout. Mouse buttons use `Mouse0` (left) / `Mouse2` (right).
 *
 * UI_SPEC 5.2 is the authority: sprint = Left Shift (held). The walk modifier (`aux` bit 0, GAMEDESIGN 9.1) has no
 * default key in UI_SPEC, so it is unbound by default and rebindable; the M0 debug key is CapsLock (held)
 * (PLAN_ISSUES PI-023 ruling; the "Shift = walk" wording of the M0-022 task card is void).
 */
import { AUX_WALK, BUTTONS, MOVE_MAX } from './buttons';

export type Action = 'forward' | 'back' | 'left' | 'right' | 'walk' | 'sprint' | 'crouch' | 'jump' | 'fire' | 'aim' | 'reload' | 'vehicle_seat';

export const DEFAULT_KEYMAP: Readonly<Record<string, Action>> = {
  KeyW: 'forward',
  KeyS: 'back',
  KeyA: 'left',
  KeyD: 'right',
  ShiftLeft: 'sprint',
  CapsLock: 'walk', // M0 debug binding only; no UI_SPEC default (rebindable, M2-041)
  KeyC: 'crouch',
  Space: 'jump',
  KeyR: 'reload',
  KeyF: 'vehicle_seat',
  Mouse0: 'fire',
  Mouse2: 'aim',
};

/** Action -> `InputCmd.buttons` bit. `walk` is an `aux` bit, `forward/back/left/right` are axes. */
export const ACTION_BUTTON: Readonly<Partial<Record<Action, number>>> = {
  sprint: BUTTONS.SPRINT,
  crouch: BUTTONS.CROUCH,
  jump: BUTTONS.JUMP,
  fire: BUTTONS.FIRE,
  aim: BUTTONS.AIM,
  reload: BUTTONS.RELOAD,
  vehicle_seat: BUTTONS.VEHICLE_SEAT,
};

export interface MoveAxes {
  moveX: number;
  moveZ: number;
}

/** Axis values for a set of held actions: forward +z, right +x; diagonals are scaled to |move| <= 127. */
export function moveAxes(held: ReadonlySet<Action>, out: MoveAxes = { moveX: 0, moveZ: 0 }): MoveAxes {
  let x = (held.has('right') ? 1 : 0) - (held.has('left') ? 1 : 0);
  let z = (held.has('forward') ? 1 : 0) - (held.has('back') ? 1 : 0);
  if (x !== 0 && z !== 0) {
    const s = MOVE_MAX / Math.hypot(x, z);
    x = Math.trunc(x * s);
    z = Math.trunc(z * s);
  } else {
    x *= MOVE_MAX;
    z *= MOVE_MAX;
  }
  out.moveX = x;
  out.moveZ = z;
  return out;
}

export function buttonsFor(held: ReadonlySet<Action>): number {
  let b = 0;
  for (const a of held) b |= ACTION_BUTTON[a] ?? 0;
  return b;
}

export function auxFor(held: ReadonlySet<Action>): number {
  return held.has('walk') ? AUX_WALK : 0;
}
