import { describe, it, expect } from 'vitest';
import * as D from './downed-state';
import en from '../ui/i18n/en.json';
import zh from '../ui/i18n/zh-Hans.json';

describe('phase machine', () => {
  it('alive→downed→alive, downed→dead→spectating→alive; illegal events ignored', () => {
    expect(D.downedNext('alive', 'down')).toBe('downed');
    expect(D.downedNext('downed', 'revive')).toBe('alive');
    expect(D.downedNext('downed', 'die')).toBe('dead');
    expect(D.downedNext('dead', 'spectate')).toBe('spectating');
    expect(D.downedNext('spectating', 'respawn')).toBe('alive');
    expect(D.downedNext('alive', 'revive')).toBe('alive');
    expect(D.downedNext('downed', 'spectate')).toBe('downed');
  });
});

describe('bleed-out, give-up, call cooldown', () => {
  it('GD §7.2: 40 HP at 0.5 HP/s is 80 s', () => {
    expect(D.bleedoutRemainingS(40)).toBe(80);
    expect(D.bleedoutRemainingS(-3)).toBe(0);
    expect(D.bleedoutFraction(20)).toBe(0.5);
    expect(D.bleedoutRemainingS(40, { ...D.DEFAULT_DOWNED_RULES, bleedHpPerS: 1 })).toBe(40);
  });
  it('give-up disabled for 5 s with a reason, enabled after; reconnect disables it', () => {
    expect(D.giveUpState(2, true)).toEqual({ enabled: false, reasonKey: 'ui.downed.reason.giveUpWait', waitS: 3 });
    expect(D.giveUpState(5, true)).toEqual({ enabled: true, waitS: 0 });
    expect(D.giveUpState(30, false).reasonKey).toBe('ui.downed.reason.reconnecting');
    expect(D.giveUpProgress(1.5)).toBe(0.5);
    expect(D.giveUpProgress(9)).toBe(1);
  });
  it('call cooldown 5 s', () => {
    expect(D.callCooldownLeftS(undefined, 10)).toBe(0);
    expect(D.callCooldownLeftS(10, 12.5)).toBe(3);
    expect(D.callCooldownLeftS(10, 15)).toBe(0);
  });
});

describe('nearest medic', () => {
  const M = (id: number, x: number, z: number, o: Partial<D.MedicInfo> = {}): D.MedicInfo => ({ id, x, z, alive: true, downed: false, enRoute: false, ...o });
  it('skips dead/downed medics; bearing 0 = north', () => {
    const r = D.nearestMedic(0, 0, [M(1, 5, 0, { downed: true }), M(2, 0, 30), M(3, 40, 0), M(4, 1, 1, { alive: false })])!;
    expect(r.medic.id).toBe(2);
    expect(r.distM).toBe(30);
    expect(r.bearingRad).toBeCloseTo(0);
    expect(D.nearestMedic(0, 0, [])).toBeNull();
  });
});

describe('killer info (acceptance 3)', () => {
  const info: D.KillerInfo = { name: 'Kurt', weapon: 3, distM: 41.6, isBot: false, friendly: false, hidden: false };
  it('no killer data at all while downed', () => {
    expect(D.killerView('downed', info, false)).toBeNull();
    expect(D.killerView('alive', info, false)).toBeNull();
  });
  it('dead shows name, weapon, rounded distance', () => {
    expect(D.killerView('dead', info, false)).toEqual({ name: 'Kurt', weapon: 3, distM: 42, tagKeys: [] });
  });
  it('hide-names setting or server hidden flag removes the name; bot and friendly tags', () => {
    expect(D.killerView('dead', info, true)).toEqual({ weapon: 3, distM: 42, tagKeys: [] });
    expect(D.killerView('dead', { ...info, hidden: true }, false)!.name).toBeUndefined();
    expect(D.killerView('dead', { ...info, isBot: true, friendly: true }, false)!.tagKeys).toEqual(['ui.downed.tag.ai', 'ui.downed.tag.friendlyFire']);
  });
});

describe('spectate', () => {
  it('cycles both directions, starts at an end, empty → undefined; not allowed while downed', () => {
    expect(D.nextSpectateTarget(undefined, [4, 5, 6])).toBe(4);
    expect(D.nextSpectateTarget(6, [4, 5, 6])).toBe(4);
    expect(D.nextSpectateTarget(4, [4, 5, 6], -1)).toBe(6);
    expect(D.nextSpectateTarget(9, [4, 5, 6])).toBe(4);
    expect(D.nextSpectateTarget(4, [])).toBeUndefined();
    expect(D.canSpectate('downed')).toBe(false);
    expect(D.canSpectate('dead')).toBe(true);
  });
});

describe('i18n keys', () => {
  it.each(['ui.downed.reason.reconnecting', 'ui.downed.reason.giveUpWait', 'ui.downed.tag.ai', 'ui.downed.tag.friendlyFire', 'ui.downed.title', 'ui.downed.noMedic', 'ui.downed.callMedic', 'ui.downed.giveUp', 'ui.dead.title', 'ui.dead.killedBy', 'ui.dead.redeploy'])('%s', (k) => {
    expect(k in en && k in zh).toBe(true);
  });
});
