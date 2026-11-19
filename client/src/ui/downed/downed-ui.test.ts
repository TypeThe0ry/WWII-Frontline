import { signal } from '@preact/signals';
import { describe, expect, it, vi } from 'vitest';
import { DEFAULT_DOWNED_RULES, type KillerInfo } from '../../game/downed-state';
import { createDownedUi, type DownedActions, type DownedInputs, type DownedOptions } from './downed-ui';

function fixture(actions: DownedActions = {}, inputs: DownedInputs = {}, rules?: DownedOptions['rules']) {
  let at = 0;
  const connected = signal(true);
  const store = createDownedUi({ actions, inputs: { ...inputs, connected }, rules, now: () => at });
  const advance = (ms: number) => { at += ms; store.update(); };
  return { store, connected, advance };
}

const KILLER: KillerInfo = { name: 'Sasha', weapon: 3, distM: 41.6, friendly: true, hidden: false, isBot: true };

describe('Downed/death UI controller', () => {
  it('keeps unsupported data and actions absent, and never treats HP as a phase dispatch', () => {
    const downedHp = signal<number | undefined>(undefined);
    const store = createDownedUi({ inputs: { downedHp } });
    downedHp.value = 2;
    expect(store.phase.value).toBe('alive');
    expect(store.bleedout.value).toBeUndefined();
    store.transition('down');
    expect(store.bleedout.value).toEqual({ seconds: 4, fraction: 0.05 });
    downedHp.value = 0;
    expect(store.phase.value).toBe('downed');
    expect(store.bleedout.value?.seconds).toBe(0);
    expect(store.medic.value).toBeUndefined();
    expect(store.callMedicState.value).toMatchObject({ enabled: false, available: false });
    expect(store.callMedic()).toBe(false);
    expect(store.beginGiveUp()).toBe(false);
    store.transition('die');
    expect(store.respawnSeconds.value).toBeUndefined();
    expect(store.redeploy()).toBe(false);
    expect(store.cycleSpectate(1)).toBe(false);
    store.dispose();
  });

  it('starts the medic cooldown only when dispatch succeeds, and checks the deadline at the action', () => {
    const callMedic = vi.fn<() => boolean>().mockReturnValueOnce(false).mockReturnValue(true);
    const { store, advance, connected } = fixture({ callMedic });
    expect(store.callMedic()).toBe(false);
    store.transition('down');
    expect(store.callMedic()).toBe(false);
    expect(store.callMedicState.value.cooldownS).toBe(0);
    expect(store.callMedic()).toBe(true);
    expect(store.callMedicState.value.cooldownS).toBe(5);
    advance(4900);
    expect(store.callMedic()).toBe(false);
    expect(store.callMedicState.value.cooldownS).toBe(1);
    advance(99);
    expect(store.callMedic()).toBe(false);
    advance(1);
    expect(store.callMedic()).toBe(true);
    expect(callMedic).toHaveBeenCalledTimes(3);
    advance(5000);
    connected.value = false;
    expect(store.callMedic()).toBe(false);
    store.dispose();
  });

  it('requires an uninterrupted hold after the wait and neither repeats nor claims death', () => {
    const giveUp = vi.fn(() => true);
    const { store, advance } = fixture({ giveUp });
    store.transition('down');
    expect(store.giveUpState.value).toMatchObject({ enabled: false, waitS: 5 });
    advance(4999);
    expect(store.beginGiveUp()).toBe(false);
    advance(1);
    expect(store.beginGiveUp()).toBe(true);
    expect(store.beginGiveUp()).toBe(false);
    advance(1500);
    expect(store.giveUpProgress.value).toBe(0.5);
    store.endGiveUp();
    advance(1500);
    expect(giveUp).not.toHaveBeenCalled();
    expect(store.beginGiveUp()).toBe(true);
    advance(2999);
    expect(giveUp).not.toHaveBeenCalled();
    advance(1);
    advance(5000);
    expect(giveUp).toHaveBeenCalledTimes(1);
    expect(store.phase.value).toBe('downed');
    expect(store.giveUpProgress.value).toBeUndefined();
    store.dispose();
  });

  it('cancels on disconnect and on lifecycle transitions; honors non-default rule timings', () => {
    const giveUp = vi.fn(() => true);
    const { store, connected, advance } = fixture({ giveUp }, {}, { ...DEFAULT_DOWNED_RULES, giveUpAfterS: 1, giveUpHoldS: 2 });
    store.transition('down');
    advance(1000);
    expect(store.beginGiveUp()).toBe(true);
    advance(1000);
    connected.value = false;
    expect(store.giveUpState.value.reasonKey).toBe('ui.downed.reason.reconnecting');
    connected.value = true;
    advance(1000);
    expect(giveUp).not.toHaveBeenCalled();
    expect(store.beginGiveUp()).toBe(true);
    store.transition('revive');
    advance(2000);
    expect(store.phase.value).toBe('alive');
    expect(giveUp).not.toHaveBeenCalled();
    store.transition('down');
    advance(1000);
    expect(store.beginGiveUp()).toBe(true);
    advance(2000);
    expect(giveUp).toHaveBeenCalledOnce();
    store.dispose();
    expect(store.beginGiveUp()).toBe(false);
  });

  it('discards early killer info and exposes only permitted fields after death', () => {
    const hideNames = signal(false);
    const { store } = fixture({}, { hideNames });
    store.transition('down');
    expect(store.setKiller(KILLER)).toBe(false);
    expect(store.killer.value).toBeNull();
    store.transition('die');
    expect(store.killer.value).toBeNull();
    expect(store.setKiller(KILLER)).toBe(true);
    expect(store.killer.value).toEqual({ name: 'Sasha', weapon: 3, distM: 42, tagKeys: ['ui.downed.tag.ai', 'ui.downed.tag.friendlyFire'] });
    hideNames.value = true;
    expect(store.killer.value).toEqual({ weapon: 3, distM: 42, tagKeys: [] });
    store.transition('respawn');
    store.transition('die'); // direct fatality on the next life
    expect(store.killer.value).toBeNull();
    store.dispose();
  });

  it('gates post-death adapters and uses only host-supplied AOI teammates and server ticks', () => {
    const spectate = vi.fn<(id: number) => boolean>().mockReturnValueOnce(false).mockReturnValue(true);
    const redeploy = vi.fn(() => true);
    const spectateCandidates = signal<readonly number[]>([3, 7]);
    const respawnClock = signal({ spawnTick: 240, nowTick: 120, tickRate: 60 });
    const { store } = fixture({ spectate, redeploy }, { spectateCandidates, respawnClock });
    store.transition('down');
    expect(store.cycleSpectate(1)).toBe(false);
    expect(store.redeploy()).toBe(false);
    expect(store.respawnSeconds.value).toBeUndefined();
    store.transition('die');
    expect(store.respawnSeconds.value).toBe(2);
    respawnClock.value = { ...respawnClock.value, nowTick: 181 };
    expect(store.respawnSeconds.value).toBe(1);
    expect(store.cycleSpectate(1)).toBe(false);
    expect(store.cycleSpectate(1)).toBe(true);
    expect(store.cycleSpectate(1)).toBe(true);
    expect(spectate.mock.calls).toEqual([[3], [3], [7]]);
    expect(store.phase.value).toBe('dead');
    expect(store.redeploy()).toBe(true);
    expect(store.phase.value).toBe('dead');
    spectateCandidates.value = [];
    expect(store.canCycleSpectate.value).toBe(false);
    expect(store.cycleSpectate(1)).toBe(false);
    expect(store.transition('spectate')).toBe(true); // host acknowledgement
    expect(store.transition('respawn')).toBe(true);
    store.dispose();
  });

  it('distinguishes a known empty medic roster from missing data and ignores incapacitated medics', () => {
    const medics = signal<NonNullable<DownedInputs['medics']>['value']>(undefined);
    const { store } = fixture({}, { medics });
    store.transition('down');
    expect(store.medic.value).toBeUndefined();
    medics.value = { px: 0, pz: 0, yawRad: Math.PI / 2, candidates: [] };
    expect(store.medic.value).toBeNull();
    medics.value = { ...medics.value, candidates: [
      { id: 5, x: 0, z: 1, alive: true, downed: true, enRoute: false },
      { id: 6, x: 7, z: 0, alive: true, downed: false, enRoute: true },
    ] };
    expect(store.medic.value).toEqual({ id: 6, distanceM: 7, bearingRad: Math.PI / 2, relativeBearingRad: 0, enRoute: true });
    store.dispose();
  });
});
