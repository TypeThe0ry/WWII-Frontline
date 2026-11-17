/**
 * Default key bindings (constants; rebinding is M2-041). `KeyboardEvent.code` based, so they do not
 * depend on the keyboard layout. Mouse buttons use `Mouse0` (left) / `Mouse2` (right).
 *
 * UI_SPEC 5.2 binds sprint to Left Shift, while the task card and GAMEDESIGN 9.1 (sim `AUX_WALK`) make
 * Shift the walk modifier; the task acceptance is explicit, so Shift = walk here and sprint sits on
 * Right Shift until the rebinding UI exists (PLAN_ISSUES PI-021).
 */
import { AUX_WALK, BUTTONS, MOVE_MAX } from './buttons';

export type Action = 'forward' | 'back' | 'left' | 'right' | 'walk' | 'sprint' | 'crouch' | 'jump' | 'fire' | 'aim' | 'reload';

export const DEFAULT_KEYMAP: Readonly<Record<string, Action>> = {
  KeyW: 'forward',
  KeyS: 'back',
  KeyA: 'left',
  KeyD: 'right',
  ShiftLeft: 'walk',
  ShiftRight: 'sprint',
  KeyC: 'crouch',
  Space: 'jump',
  KeyR: 'reload',
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
