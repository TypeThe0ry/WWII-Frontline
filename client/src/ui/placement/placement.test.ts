import { describe, expect, it, vi } from 'vitest';
import { BUTTONS } from '../../input/buttons';
import { PLACE_ERROR_KEY, placementNext, type PlacementResult } from '../../game/placement';
import { PlacementActions, SeatInputAdapter, encodeSquadToolCommand, requestSeatSwapForKey } from '../../game/placement-actions';
import type { PlacementChecker, PlacementReply } from '../../game/placement-actions';
import { SeatMachine } from '../../game/seat';
import { PlacementGhostLabel, VehicleEnterPrompt, VehicleSeatHud } from './index';

const pos = [320, 5, 30] as const;
const checkResult = (result: PlacementResult): PlacementChecker => (state) => placementNext(state, { t: 'check', result });
const valid = checkResult({ legal: true, supplyCost: 50 });

function text(vnode: { props: { children?: unknown } }): string {
  const children = vnode.props.children;
  function walk(value: unknown): string {
    if (typeof value === 'string' || typeof value === 'number') return String(value);
    if (Array.isArray(value)) return value.map(walk).join('');
    if (typeof value === 'object' && value && 'props' in value) return walk((value as typeof vnode).props.children);
    return '';
  }
  return walk(children);
}

it.each(['EnemyNear', 'Slope', 'Water', 'TooCloseOwnGarrison', 'NoSupplies', 'TeamCap', 'TooCloseObjective', 'TooCloseHq'] as const)(
  'displays the localized %s result from the preview adapter',
  (reason) => {
    const preview = { legal: false, reasonKey: PLACE_ERROR_KEY[reason] };
    const label = PlacementGhostLabel({ preview, translate: (key) => `translation:${key}` });
    const expected = reason === 'NoSupplies' ? ['grey', 'dotted', 'minus'] : ['warn', 'hatched', 'warn'];
    expect(['color', 'pattern', 'icon'].map((name) => label.props[`data-placement-${name}`])).toEqual(expected);
    expect(text(label)).toContain(`translation:${PLACE_ERROR_KEY[reason]}`);
  },
);

it('shows resource counts only when provided, and fails closed with missing facts or non-finite points', () => {
  const actions = new PlacementActions(valid);
  actions.enter('garrison');
  actions.update(pos);
  const label = PlacementGhostLabel({ preview: actions.view.value.preview!, capacity: { used: 0, limit: 6 }, nearbySupplies: 75 });
  expect(label.props['data-placement-color']).toBe('blue');
  expect(text(label)).toMatch(/Valid placement.*Cost: 50.*Nearby Supplies: 75.*Placed: 0\/6/);
  actions.update([1e100, 0, 0]);
  expect(actions.view.value.preview?.legal).toBe(false);
  const noFacts = new PlacementActions();
  noFacts.enter('garrison');
  noFacts.update(pos);
  expect(noFacts.view.value.preview).toEqual({ legal: false, reasonKey: 'ui.officer.err.previewUnavailable' });
});

describe('placement commands', () => {
  it('does not send without a legal current target; rechecks on commit', async () => {
    const send = vi.fn(async (): Promise<PlacementReply> => ({ ok: true }));
    const check = vi.fn(valid);
    const actions = new PlacementActions(check, send);
    actions.enter('garrison');
    await actions.commit();
    actions.update(pos);
    check.mockImplementation(checkResult({ legal: false, reasonKey: PLACE_ERROR_KEY.EnemyNear }));
    await actions.commit();
    expect(send).not.toHaveBeenCalled();
    expect(actions.view.value.state.mode).toBe('invalid');
    actions.cancel();
    expect(actions.view.value.state.mode).toBe('idle');
  });

  it('dispatches a legal ray hit once and preserves an authoritative rejection on that target', async () => {
    let resolve!: (reply: PlacementReply) => void;
    const send = vi.fn(() => new Promise<PlacementReply>((done) => { resolve = done; }));
    const actions = new PlacementActions(valid, send);
    actions.enter('garrison');
    actions.update(pos);
    const pending = actions.commit();
    expect(actions.view.value.state.mode).toBe('committing');
    await actions.commit();
    expect(send).toHaveBeenCalledExactlyOnceWith({ kind: 'garrison', pos });
    const reply = { ok: false, reasonKey: PLACE_ERROR_KEY.NoSupplies } as const;
    resolve(reply);
    await expect(pending).resolves.toEqual(reply);
    expect(actions.view.value.state.mode).toBe('placing');
    actions.update([...pos]);
    await actions.commit();
    expect(send).toHaveBeenCalledTimes(1);
    expect(actions.view.value.preview).toMatchObject({ legal: false, reasonKey: reply.reasonKey });
    expect(PlacementGhostLabel({ preview: actions.view.value.preview!, translate: (key) => key }).props['data-placement-color']).toBe('grey');
    actions.update([321, 5, 30]);
    expect(actions.view.value.preview?.legal).toBe(true);
  });

  it('does not send a world position for a crate and clears on acceptance', async () => {
    const send = vi.fn(async (): Promise<PlacementReply> => ({ ok: true }));
    const actions = new PlacementActions(valid, send);
    actions.enter('supply_crate');
    actions.update(pos);
    await actions.commit();
    expect(send).toHaveBeenCalledExactlyOnceWith({ kind: 'supply_crate' });
    expect(actions.view.value.state.mode).toBe('idle');
  });
});

const { USE, FIRE, VEHICLE_SEAT } = BUTTONS;
const onFoot = { canBoard: true, inVehicle: false, fDown: true, heldS: 0 };

it('keeps ordinary interact intact and sends one boarding pulse on a near-vehicle tap', () => {
  const adapter = new SeatInputAdapter();
  expect(adapter.sample(USE | FIRE, { ...onFoot, canBoard: false }).buttons).toBe(USE | FIRE);
  expect(adapter.sample(USE, onFoot).buttons).toBe(USE); // Don't hijack an interaction mid-hold.
  expect(adapter.sample(0, { ...onFoot, fDown: false }).buttons).toBe(0);
  expect(adapter.sample(USE | FIRE, onFoot).buttons).toBe(FIRE);
  const release = { ...onFoot, fDown: false, heldS: 0.2 };
  expect(adapter.sample(FIRE, release)).toEqual({ buttons: FIRE | VEHICLE_SEAT, openSeatMenu: false });
  expect(adapter.sample(FIRE, release).buttons).toBe(FIRE);
});

it('opens the seat menu once on hold and never emits a boarding pulse on release', () => {
  const adapter = new SeatInputAdapter();
  adapter.sample(USE, onFoot);
  const hold = { ...onFoot, heldS: 0.35 };
  expect(adapter.sample(USE, hold)).toEqual({ buttons: 0, openSeatMenu: true });
  expect(adapter.sample(USE, hold)).toEqual({ buttons: 0, openSeatMenu: false });
  expect(adapter.sample(0, { ...hold, fDown: false }).buttons).toBe(0);
});

it('starts exit on the first held tick, clears on release, and does not reboard after ejection', () => {
  const adapter = new SeatInputAdapter();
  const inVehicle = { ...onFoot, inVehicle: true };
  expect(adapter.sample(USE, inVehicle).buttons).toBe(VEHICLE_SEAT);
  expect(adapter.sample(USE, { ...inVehicle, heldS: 0.5 }).buttons).toBe(VEHICLE_SEAT);
  expect(adapter.sample(0, { ...inVehicle, fDown: false }).buttons).toBe(0);
  expect(adapter.sample(USE, inVehicle).buttons).toBe(VEHICLE_SEAT);
  expect(adapter.sample(USE, onFoot).buttons).toBe(0);
  expect(adapter.sample(0, { ...onFoot, fDown: false }).buttons).toBe(0);
});

it.each([0, 60])('dispatches digit-key swaps only with a real port and uses the existing %i-tick delay', (delay) => {
  const seat = new SeatMachine();
  seat.confirm('driver', 142);
  const send = vi.fn();
  const input = { key: '2', seats: ['driver', 'gunner', 'commander'] as const, tick: 10, speedKmh: delay > 0 ? 20 : 0 };
  expect(requestSeatSwapForKey(input, seat)).toBe(false);
  expect(seat.state.pending).toBeUndefined();
  expect(requestSeatSwapForKey(input, seat, send)).toBe(true);
  expect(send).toHaveBeenCalledExactlyOnceWith({ kind: 'swap', vehicleId: 142, seat: 'gunner' });
  expect(seat.state).toMatchObject({ seat: 'driver', pending: 'swap', readyAt: 10 + delay });
  expect(requestSeatSwapForKey(input, seat, send)).toBe(false);
  const hud = VehicleSeatHud({ state: seat.state, seats: [], tick: 10, tickHz: 30 })!;
  expect(text(hud)).toContain(delay > 0 ? 'Switching to Gunner · 2.0 s' : 'Switching to Gunner · Awaiting confirmation');
  expect(text(hud)).not.toContain('Number keys');
});

it('renders bailout and does not advertise boarding when nearest-seat entry is unavailable', () => {
  expect(text(VehicleSeatHud({ state: { seat: 'none' }, seats: [], bailout: true })!)).toContain('Bailout');
  expect(text(VehicleEnterPrompt({ vehicleId: 128, name: 'Tank', seats: [], canBoardNearest: false }))).not.toContain('F: Enter');
});


it('encodes supported squad operations as C0 payloads', () => {
  expect([...encodeSquadToolCommand({ kind: 'lock', locked: true })]).toEqual([0x12, 7, 0, 0, 0]);
  expect([...encodeSquadToolCommand({ kind: 'lock', locked: false })]).toEqual([0x12, 8, 0, 0, 0]);
  expect([...encodeSquadToolCommand({ kind: 'kick', playerId: 4 })]).toEqual([0x12, 4, 0, 4, 0]);
  expect([...encodeSquadToolCommand({ kind: 'promote', playerId: 7 })]).toEqual([0x12, 5, 0, 7, 0]);
  expect(() => encodeSquadToolCommand({ kind: 'kick', playerId: 256 })).toThrow(RangeError);
});
