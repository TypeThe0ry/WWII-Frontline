import { signal } from '@preact/signals';
import { describe, expect, it } from 'vitest';
import { EventBus } from '../../game/event-bus';
import { GameState } from '../../game/state';
import { GameState as ReliableState } from '../../game/state/game-state';
import { createDownedUi } from '../downed';
import { createHudStore, type HudAmmoContext, type HudInputs } from './hud-signals';

function fixture(inputs: HudInputs = {}) {
  const bus = new EventBus();
  const game = new GameState();
  const reliable = new ReliableState(bus);
  let at = 0;
  const store = createHudStore({ bus, game, selfId: 1, roster: reliable.roster, inputs, now: () => at });
  const advance = (ms: number) => { at += ms; store.update(); };
  return { bus, game, reliable, store, advance };
}

describe('HUD state bridge', () => {
  it('consumes real counters together at most 20 Hz and never guesses ammo capacity', () => {
    const ammoContext = signal<HudAmmoContext | undefined>(undefined);
    const { game, store, advance } = fixture({ ammoContext });
    expect(store.ammo.value).toBeUndefined();
    expect(store.healthBand.value).toBeUndefined();
    game.net.health.value = 63;
    game.net.stamina.value = 12;
    game.net.ammoAuth.value = 10;
    game.net.reserveAuth.value = 15;
    game.net.snapshotTick.value = 10;
    game.local.stance = 1;
    game.local.yaw = Math.PI / 2;
    advance(49);
    expect(store.ammo.value).toBeUndefined();
    expect(store.heading.value).toBe(90);
    advance(1);
    expect(store.ammo.value).toEqual({ tier: undefined, emptyMag: false, exact: undefined });
    expect(store.healthBand.value).toBe(3);
    expect(store.fx.value.state).toBe('clear'); // critical health does not prove bleeding
    expect(store.stamina.value).toEqual({ value: 12 / 63, warn: true });
    expect(store.stance.value).toEqual({ value: 1, source: 'predicted' });
    ammoContext.value = { initialTotal: 100 };
    expect(store.ammo.value?.tier).toBe('low');
    expect(store.ammo.value?.exact).toBeUndefined();
    ammoContext.value = { initialTotal: 100, nearCrate: true };
    expect(store.ammo.value?.exact).toEqual({ mag: 10, reserve: 15 });
    game.net.ammoAuth.value = 0;
    game.net.snapshotTick.value++;
    advance(49);
    expect(store.ammo.value?.emptyMag).toBe(false);
    advance(1);
    expect(store.ammo.value?.emptyMag).toBe(true);
    store.dispose();
  });

  it('shows only confirmed hits, decays a view-relative wedge, and follows shared phase for FX', () => {
    const downed = createDownedUi();
    const bleeding = signal(false);
    const reduceMotion = signal(true);
    const { bus, store, advance } = fixture({ phase: downed.phase, bleeding, reduceMotion });
    bus.emit('fire.local', { weapon: 1, muzzlePos: { x: 0, y: 0, z: 0 }, dir: { x: 0, y: 0, z: 1 } });
    expect(store.hit.value).toBeUndefined();
    bus.emit('hit.confirm', { target: 2, zone: 0, damage: 0.5, killed: false });
    bus.emit('damage.taken', { angle: 1.5, amount: 0.5 });
    bus.emit('suppress', { angle: 0, intensity: 0.5 });
    expect(store.hit.value).toBe('headshot');
    expect(store.fx.value).toEqual({ state: 'suppressed', intensity: 0.5, reducedMotion: true });
    advance(79);
    expect(store.hit.value).toBe('headshot');
    advance(1);
    expect(store.hit.value).toBeUndefined();
    advance(220);
    expect(store.damage.value).toEqual({ angleRad: 1.5, opacity: 0.5 });
    advance(300);
    expect(store.damage.value).toBeUndefined();
    bleeding.value = true;
    expect(store.fx.value.state).toBe('bleeding');
    downed.transition('down');
    expect(store.fx.value.state).toBe('downed');
    bleeding.value = false;
    downed.transition('revive');
    expect(store.fx.value.state).toBe('suppressed');
    advance(2400);
    expect(store.fx.value.state).toBe('clear');
    store.dispose();
    downed.dispose();
    expect(bus.listenerCount('hit.confirm')).toBe(0);
    bus.emit('hit.confirm', { target: 2, zone: 0, damage: 0.5, killed: false });
    expect(store.hit.value).toBeUndefined();
  });

  it('uses C4 name and time data, caps the visible feed, and drops local victim detail while downed', () => {
    const phase = signal<'alive' | 'downed'>('alive');
    const serverClock = signal({ nowTick: 120, tickRate: 60 });
    const { store, reliable, advance } = fixture({ phase, serverClock });
    for (const [id, nick] of [[1, 'Local'], [2, 'Teammate']] as const) {
      reliable.apply({ type: 'roster', player: id, seq: 0, team: 1, squad: 1, role: 2, flags: 0, nick });
    }
    reliable.apply({ type: 'phase', phase: 1, endsAt: 240 });
    expect(store.matchSeconds.value).toBe(2);
    serverClock.value = { nowTick: 181, tickRate: 60 };
    expect(store.matchSeconds.value).toBe(1);
    for (const victim of [2, 3, 4, 5]) reliable.apply({ type: 'kill', killer: 1, victim, weapon: 3, headshot: false, distQ: 4 });
    expect(store.feed.value.map((e) => e.victim)).toEqual([2, 3, 4]);
    expect(store.feed.value[0]).toMatchObject({ killerName: 'Local', victimName: 'Teammate', friendlyFire: true, local: true });
    phase.value = 'downed';
    reliable.apply({ type: 'kill', killer: 7, victim: 1, weapon: 3, headshot: false, distQ: 4 });
    advance(4999);
    expect(store.feed.value).toHaveLength(3);
    advance(1);
    expect(store.feed.value.map((e) => e.victim)).toEqual([5]);
    phase.value = 'alive';
    expect(store.feed.value.map((e) => e.victim)).toEqual([5]);
    advance(5000);
    expect(store.feed.value).toHaveLength(0);
    store.dispose();
  });

  it('hides exact squad distance after five seconds without inventing a new TEAM update', () => {
    const squad = signal({ members: [{ id: 1, name: 'Joe', distanceM: 12 }], teamUpdatedAtMs: 0 });
    const { store, advance } = fixture({ squad });
    advance(5000);
    expect(store.squadFresh.value).toBe(true);
    advance(1);
    expect(store.squadFresh.value).toBe(false);
    squad.value = { ...squad.value, teamUpdatedAtMs: 5001 };
    advance(0);
    expect(store.squadFresh.value).toBe(true);
    store.dispose();
  });
});
