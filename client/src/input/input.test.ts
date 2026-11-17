import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { AUX_WALK, BUTTONS, MOVE_MAX } from './buttons';
import { DEFAULT_KEYMAP, moveAxes, type Action } from './keymap';
import { PointerLockController, type LockDocument, type LockTarget } from './pointer-lock';
import { InputSampler, pitchFromCode, quantPitch, quantYaw, yawFromCode } from './sampler';

const ROOT = fileURLToPath(new URL('../../../', import.meta.url));

describe('buttons table equals the Rust protocol constants (no copied numbers)', () => {
  const src = readFileSync(`${ROOT}crates/protocol/src/msg/input.rs`, 'utf8');
  const mod = /pub mod buttons \{([\s\S]*?)\n\}/.exec(src)?.[1] ?? '';
  const rust = new Map([...mod.matchAll(/pub const (\w+): u16 = 1 << (\d+);/g)].map((m) => [m[1] as string, 1 << Number(m[2])]));
  it('every Rust button has the same bit here, and vice versa', () => {
    expect(rust.size).toBe(14);
    expect(Object.fromEntries(rust)).toEqual({ ...BUTTONS });
  });
  it('AUX_WALK equals the sim constant', () => {
    const sim = readFileSync(`${ROOT}crates/sim/src/movement/mod.rs`, 'utf8');
    expect(/pub const AUX_WALK: u8 = (?:1 << )?(\w+);/.exec(sim)).not.toBeNull();
    expect(AUX_WALK).toBe(1);
  });
});

const held = (...a: Action[]): ReadonlySet<Action> => new Set(a);

describe('keyboard -> InputCmd mapping', () => {
  it('W = forward (+127 on z), S = back, D = +x, A = -x', () => {
    expect(moveAxes(held('forward'))).toEqual({ moveX: 0, moveZ: 127 });
    expect(moveAxes(held('back'))).toEqual({ moveX: 0, moveZ: -127 });
    expect(moveAxes(held('right'))).toEqual({ moveX: 127, moveZ: 0 });
    expect(moveAxes(held('left'))).toEqual({ moveX: -127, moveZ: 0 });
    expect(moveAxes(held('forward', 'back'))).toEqual({ moveX: 0, moveZ: 0 });
  });
  it('diagonals are normalised to |move| <= 127 and never use -128', () => {
    for (const combo of [['forward', 'right'], ['forward', 'left'], ['back', 'right'], ['back', 'left']] as Action[][]) {
      const m = moveAxes(held(...combo));
      expect(Math.hypot(m.moveX, m.moveZ)).toBeLessThanOrEqual(MOVE_MAX);
      expect(Math.hypot(m.moveX, m.moveZ)).toBeGreaterThan(MOVE_MAX * 0.98);
      expect(Math.min(m.moveX, m.moveZ)).toBeGreaterThanOrEqual(-127);
    }
  });
  it('Left Shift = sprint (UI_SPEC 5.2), CapsLock = walk (aux bit 0, M0 debug key), buttons use the protocol bit positions', () => {
    const s = new InputSampler();
    s.press('ShiftLeft');
    s.press('KeyC');
    s.press('Space');
    s.press('Mouse0');
    s.press('Mouse2');
    s.press('KeyR');
    const c = s.build(1, 10, 6);
    expect(c.aux).toBe(0);
    expect(c.buttons).toBe(BUTTONS.SPRINT | BUTTONS.CROUCH | BUTTONS.JUMP | BUTTONS.FIRE | BUTTONS.AIM | BUTTONS.RELOAD);
    expect(c.weaponSlot).toBe(0);
    s.press('CapsLock');
    expect(s.build(2, 11, 6).aux).toBe(AUX_WALK);
    s.release('CapsLock');
    s.release('ShiftLeft');
    const d = s.build(3, 12, 6);
    expect(d.aux).toBe(0);
    expect(d.buttons & BUTTONS.SPRINT).toBe(0);
  });
  it('walk has no default key besides the CapsLock debug binding; Right Shift is unbound', () => {
    const walkKeys = Object.entries(DEFAULT_KEYMAP).filter(([, a]) => a === 'walk').map(([k]) => k);
    expect(walkKeys).toEqual(['CapsLock']);
    expect(DEFAULT_KEYMAP.ShiftRight).toBeUndefined();
    const sprintKeys = Object.entries(DEFAULT_KEYMAP).filter(([, a]) => a === 'sprint').map(([k]) => k);
    expect(sprintKeys).toEqual(['ShiftLeft']);
  });
  it('two keys on one action: the action stays held until both are up', () => {
    const km = { ...DEFAULT_KEYMAP, ArrowUp: 'forward' as const };
    const s = new InputSampler(km);
    s.press('KeyW');
    s.press('ArrowUp');
    s.release('KeyW');
    expect(s.build(1, 1, 0).moveZ).toBe(127);
    s.release('ArrowUp');
    expect(s.build(2, 2, 0).moveZ).toBe(0);
  });
  it('mouse deltas accumulate until the next build, then reset; yaw wraps, pitch clamps', () => {
    const s = new InputSampler();
    s.sensitivity = 0.5; // deg per pixel
    s.mouseMove(10, 0);
    s.mouseMove(10, 0);
    const a = s.build(1, 1, 0);
    expect(yawFromCode(a.yaw)).toBeCloseTo((10 * Math.PI) / 180, 3);
    const b = s.build(2, 2, 0);
    expect(b.yaw).toBe(a.yaw); // no new movement
    s.mouseMove(-1000, 0);
    expect(s.build(3, 3, 0).yaw).toBeGreaterThan(30000); // wrapped below 0 -> large code
    s.mouseMove(0, -100000);
    expect(pitchFromCode(s.build(4, 4, 0).pitch)).toBeCloseTo(Math.PI / 2, 2);
    s.mouseMove(0, 1e6);
    expect(pitchFromCode(s.build(5, 5, 0).pitch)).toBeCloseTo(-Math.PI / 2, 2);
    s.mouseMove(NaN, Infinity);
    expect(Number.isFinite(s.yaw)).toBe(true);
  });
  it('yaw / pitch quantisation inverts the protocol dequantisers', () => {
    for (const r of [0, 0.5, 3.14159, 6.2]) expect(Math.abs(yawFromCode(quantYaw(r)) - r)).toBeLessThan((2 * Math.PI) / 65536);
    for (const r of [-1.5, -0.3, 0, 0.7, 1.5]) expect(Math.abs(pitchFromCode(quantPitch(r)) - r)).toBeLessThan(Math.PI / 32767);
    expect(quantPitch(10)).toBe(32767);
    expect(quantPitch(-10)).toBe(-32768);
    expect(quantYaw(-0.0001)).toBe(0 | quantYaw(2 * Math.PI - 0.0001));
  });
  it('seq is masked to u16 and view_tick clamped', () => {
    const s = new InputSampler();
    expect(s.build(65536 + 5, 1, -3).seq).toBe(5);
    expect(s.build(1, 1, -3).viewTick).toBe(0);
    expect(s.build(1, 1, 1e9).viewTick).toBe(65535);
  });
});

describe('Pointer Lock', () => {
  const rig = () => {
    const canvas = Object.assign(new EventTarget(), { requestPointerLock: () => undefined }) as LockTarget;
    const doc = Object.assign(new EventTarget(), { pointerLockElement: null as unknown, hidden: false }) as LockDocument;
    const win = new EventTarget();
    const sampler = new InputSampler();
    const c = new PointerLockController(sampler, canvas, doc, win);
    c.attach();
    const fire = (t: EventTarget, type: string, props: object = {}): void => {
      t.dispatchEvent(Object.assign(new Event(type), props));
    };
    const lock = (): void => {
      doc.pointerLockElement = canvas;
      fire(doc, 'pointerlockchange');
    };
    const unlock = (): void => {
      doc.pointerLockElement = null;
      fire(doc, 'pointerlockchange');
    };
    return { canvas, doc, win, sampler, c, fire, lock, unlock };
  };

  it('keys are ignored before the lock; losing the lock clears every key bit', () => {
    const r = rig();
    r.fire(r.win, 'keydown', { code: 'KeyW' });
    expect(r.sampler.build(1, 1, 0).moveZ).toBe(0);
    r.lock();
    r.fire(r.win, 'keydown', { code: 'KeyW' });
    r.fire(r.canvas, 'mousedown', { button: 0 });
    r.fire(r.canvas, 'mousemove', { movementX: 5, movementY: 0 });
    const c = r.sampler.build(2, 2, 0);
    expect(c.moveZ).toBe(127);
    expect(c.buttons & BUTTONS.FIRE).toBe(BUTTONS.FIRE);
    r.fire(r.win, 'keydown', { code: 'KeyD' });
    r.unlock();
    const d = r.sampler.build(3, 3, 0);
    expect(d.moveZ).toBe(0);
    expect(d.moveX).toBe(0);
    expect(d.buttons).toBe(0);
    expect(r.c.lostCount).toBe(1);
  });
  it('blur and a hidden tab also release every key', () => {
    const r = rig();
    r.lock();
    r.fire(r.win, 'keydown', { code: 'KeyW' });
    r.fire(r.win, 'blur');
    expect(r.sampler.build(1, 1, 0).moveZ).toBe(0);
    r.fire(r.win, 'keydown', { code: 'KeyA' });
    (r.doc as { hidden: boolean }).hidden = true;
    r.fire(r.doc, 'visibilitychange');
    expect(r.sampler.build(2, 2, 0).moveX).toBe(0);
  });
  it('mouse movement without lock is ignored and detach() removes the listeners', () => {
    const r = rig();
    r.fire(r.canvas, 'mousemove', { movementX: 100, movementY: 100 });
    expect(r.sampler.build(1, 1, 0).yaw).toBe(0);
    r.lock();
    r.c.detach();
    r.fire(r.win, 'keydown', { code: 'KeyW' });
    expect(r.sampler.build(2, 2, 0).moveZ).toBe(0);
  });
});
