import { describe, it, expect } from 'vitest';
import { factionState, autoAssign, balanceShare, DEFAULT_RULES } from './faction-rules';
import { applyFilters, sortServers, pingRating, occupancy, joinState, rejectKey, type ServerInfo } from './server-list';
import en from '../ui/i18n/en.json';
import zh from '../ui/i18n/zh-Hans.json';

const c = (h0: number, h1: number, b0 = 0, b1 = 0) => ({ humans: [h0, h1] as [number, number], bots: [b0, b1] as [number, number] });

describe('faction rules (GAMEDESIGN §2.4)', () => {
  it('diff 3 disables the bigger team with a reason; diff 2 is selectable', () => {
    const us = factionState(0, c(13, 10));
    expect(us.state).toBe('unbalanced');
    expect(us.reasonKey).toBe('ui.faction.reason.unbalanced');
    expect(factionState(0, c(12, 10)).state).toBe('open');
    expect(factionState(1, c(13, 10)).state).toBe('open');
  });
  it('cap 50 humans → full; cooldown applies only when otherwise open', () => {
    expect(factionState(0, c(50, 50)).state).toBe('full');
    expect(factionState(0, c(49, 49)).state).toBe('open');
    expect(factionState(0, c(5, 5), DEFAULT_RULES, 30).state).toBe('cooldown');
    expect(factionState(0, c(13, 10), DEFAULT_RULES, 30).state).toBe('unbalanced');
  });
  it('rules come from the server object, not constants', () => {
    expect(factionState(0, c(11, 10), { ...DEFAULT_RULES, maxHumanDiff: 0 }).state).toBe('unbalanced');
  });
  it('auto-assign picks the smaller team, handles ties and none', () => {
    expect(autoAssign(c(5, 9))).toBe(0);
    expect(autoAssign(c(9, 5))).toBe(1);
    expect(autoAssign(c(4, 4, 6, 2))).toBe(1);
    expect(autoAssign(c(4, 4, 0, 0))).toBe(0);
    expect(autoAssign(c(50, 50))).toBeNull();
    expect(autoAssign(c(13, 10))).toBe(1);
  });
  it('balance share', () => {
    expect(balanceShare(c(0, 0))).toBe(0.5);
    expect(balanceShare(c(3, 1))).toBe(0.75);
  });
});

const S = (o: Partial<ServerInfo>): ServerInfo => ({ id: 'a', name: 'Alpha', map: 'carentan', mode: 'frontline', humans: 10, bots: 5, cap: 100, phase: 'live', pingMs: 40, region: 'eu', version: 1, locked: false, compatible: true, ...o });

describe('server list', () => {
  const list = [S({ id: 'a' }), S({ id: 'b', name: 'Bravo', pingMs: 150, humans: 70 }), S({ id: 'c', name: 'Charlie', compatible: false, pingMs: 10 }), S({ id: 'd', name: 'Delta', locked: true, bots: 0, humans: 100 }), S({ id: 'e', name: 'Echo', pingMs: undefined, humans: 0 })];
  it('filters', () => {
    expect(applyFilters(list, { compatibleOnly: true }).map((s) => s.id)).toEqual(['a', 'b', 'd', 'e']);
    expect(applyFilters(list, { maxPingMs: 100 }).map((s) => s.id)).toEqual(['a', 'c', 'd']);
    expect(applyFilters(list, { noBots: true, noPassword: true }).map((s) => s.id)).toEqual([]);
    expect(applyFilters(list, { hideFull: true, hideEmpty: true }).map((s) => s.id)).toEqual(['a', 'b', 'c']);
    expect(applyFilters(list, { keyword: ' RAV' }).map((s) => s.id)).toEqual(['b']);
  });
  it('sorts: ping, players, name, recommended (incompatible last)', () => {
    expect(sortServers(list, 'ping')[0]!.id).toBe('c');
    expect(sortServers(list, 'ping').at(-1)!.id).toBe('e');
    expect(sortServers(list, 'players')[0]!.id).toBe('d');
    expect(sortServers(list, 'name').map((s) => s.id)).toEqual(['a', 'b', 'c', 'd', 'e']);
    expect(sortServers(list, 'recommended').at(-1)!.id).toBe('c');
  });
  it('ping rating thresholds, occupancy, join state', () => {
    expect([60, 61, 120, 121, undefined].map(pingRating)).toEqual(['good', 'fair', 'fair', 'poor', 'unknown']);
    expect(occupancy(S({ humans: 72, bots: 18 }))).toBe('72+18/100');
    expect(joinState(S({ compatible: false }))).toEqual({ enabled: false, reasonKey: 'ui.browser.reason.incompatible' });
    expect(joinState(S({})).enabled).toBe(true);
  });
  it('reject codes map to keys', () => {
    expect(rejectKey(0)).toBe('ui.reject.version');
    expect(rejectKey(4)).toBe('ui.reject.dataHash');
    expect(rejectKey(99)).toBe('ui.reject.unknown');
  });
});

describe('i18n keys exist for this slice', () => {
  const keys = ['ui.faction.reason.full', 'ui.faction.reason.unbalanced', 'ui.faction.reason.cooldown', 'ui.browser.reason.incompatible', 'ui.browser.reason.full', 'ui.reject.version', 'ui.reject.full', 'ui.reject.banned', 'ui.reject.ticket', 'ui.reject.dataHash', 'ui.reject.unknown'];
  it.each(keys)('%s in en and zh-Hans', (k) => {
    expect(k in en).toBe(true);
    expect(k in zh).toBe(true);
  });
});
