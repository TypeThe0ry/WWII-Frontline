import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import * as P from './placement';
import { SeatMachine } from './seat';
import en from '../ui/i18n/en.json';
import zh from '../ui/i18n/zh-Hans.json';

describe('place errors mirror the sim enum', () => {
  it('names and order match crates/sim garrison.rs', () => {
    const src = readFileSync(new URL('../../../crates/sim/src/spawn/garrison.rs', import.meta.url).pathname, 'utf8');
    const body = /pub enum PlaceError \{([^}]*)\}/.exec(src)![1]!;
    const names = [...body.matchAll(/^\s*([A-Za-z]+),/gm)].map((m) => m[1]);
    expect(names).toEqual([...P.PLACE_ERRORS]);
  });
  it('every error maps to a key present in en and zh (acceptance 1: distinct reasons)', () => {
    const keys = Object.values(P.PLACE_ERROR_KEY);
    expect(new Set(keys).size).toBe(keys.length);
    for (const k of keys) expect(k in en && k in zh).toBe(true);
  });
});

describe('ghost style (acceptance 2: three states distinguishable by pattern and icon, not colour alone)', () => {
  it('legal / illegal / insufficient supplies', () => {
    const a = P.ghostStyle(P.resultFromError(null));
    const b = P.ghostStyle(P.resultFromError('Slope'), 'Slope');
    const c = P.ghostStyle(P.resultFromError('NoSupplies'), 'NoSupplies');
    expect([a.color, b.color, c.color]).toEqual(['blue', 'warn', 'grey']);
    expect(new Set([a, b, c].map((s) => s.pattern)).size).toBe(3);
    expect(new Set([a, b, c].map((s) => s.icon)).size).toBe(3);
  });
});

describe('placement machine (acceptance 4)', () => {
  it('place → commit → accepted returns to idle', () => {
    let s = P.placementNext(P.PLACEMENT_IDLE, { t: 'enter', kind: 'garrison' });
    s = P.placementNext(s, { t: 'commit' });
    expect(s.mode).toBe('committing');
    expect(P.placementNext(s, { t: 'accepted' })).toEqual(P.PLACEMENT_IDLE);
  });
  it('server rejection goes back to the ghost with the server reason', () => {
    let s = P.placementNext(P.PLACEMENT_IDLE, { t: 'enter', kind: 'ammo_crate' });
    s = P.placementNext(s, { t: 'commit' });
    s = P.placementNext(s, { t: 'rejected', reasonKey: 'ui.officer.err.enemyNear' });
    expect(s).toEqual({ mode: 'placing', kind: 'ammo_crate', reasonKey: 'ui.officer.err.enemyNear' });
  });
  it('illegal ghost cannot be committed; becomes placeable again when legal; cancel always works', () => {
    let s = P.placementNext(P.PLACEMENT_IDLE, { t: 'enter', kind: 'garrison' });
    s = P.placementNext(s, { t: 'check', legal: false });
    expect(s.mode).toBe('invalid');
    expect(P.placementNext(s, { t: 'commit' }).mode).toBe('invalid');
    s = P.placementNext(s, { t: 'check', legal: true });
    expect(s.mode).toBe('placing');
    expect(P.placementNext(P.placementNext(s, { t: 'commit' }), { t: 'cancel' })).toEqual(P.PLACEMENT_IDLE);
  });
});

describe('tools and costs', () => {
  it('garrison kit needs officer in a squad ≥ 2; crates need support', () => {
    expect(P.toolAvailable('garrison', 'rifleman', 4, true).reasonKey).toBe('ui.officer.tool.needOfficer');
    expect(P.toolAvailable('garrison', 'officer', 1, true).reasonKey).toBe('ui.officer.tool.needSquad');
    expect(P.toolAvailable('garrison', 'officer', 2, true).ok).toBe(true);
    expect(P.toolAvailable('ammo_crate', 'medic', 4, true).reasonKey).toBe('ui.officer.tool.needSupport');
    expect(P.toolAvailable('supply_crate', 'support', 4, true).ok).toBe(true);
    expect(P.toolAvailable('supply_crate', 'support', 4, false).ok).toBe(false);
  });
  it('costs and crate cooldown', () => {
    expect(P.garrisonCost('own')).toBe(50);
    expect(P.garrisonCost('neutral')).toBe(50);
    expect(P.garrisonCost('enemy')).toBe(100);
    expect(P.crateCooldownLeftS(100, 110)).toBe(20);
    expect(P.crateCooldownLeftS(undefined, 0)).toBe(0);
  });
});

describe('vehicle seat input (acceptance 3)', () => {
  it('F: tap enters nearest free seat, hold opens the menu, hold 0.8 s inside exits', () => {
    expect(P.fKeyAction(false, 0.1, true)).toBe('enterNearest');
    expect(P.fKeyAction(false, 0.1, false)).toBe('none');
    expect(P.fKeyAction(false, 0.4, false)).toBe('seatMenu');
    expect(P.fKeyAction(true, 0.79, false)).toBe('none');
    expect(P.fKeyAction(true, 0.8, false)).toBe('exit');
    expect(P.exitProgress(0.4)).toBeCloseTo(0.5);
  });
  it('digit maps to seat; free-seat preference', () => {
    const seats = ['driver', 'gunner', 'commander'] as const;
    expect(P.seatForKey('2', seats)).toBe('gunner');
    expect(P.seatForKey('4', seats)).toBeUndefined();
    expect(P.seatForKey('x', seats)).toBeUndefined();
    expect(P.nearestFreeSeat({ driver: true, gunner: false, commander: false })).toBe('gunner');
    expect(P.nearestFreeSeat({ driver: true, gunner: true, commander: true })).toBeUndefined();
  });
  it('one key swaps instantly when stopped; moving takes 60 ticks (2 s) with the existing SeatMachine', () => {
    const m = new SeatMachine();
    m.confirm('driver', 7);
    expect(m.requestSwap(P.seatForKey('2', ['driver', 'gunner', 'commander'])!, 100, 0)).toBe(true);
    expect(m.due(100)).toBe(true);
    m.confirm('gunner', 7);
    expect(m.requestSwap('commander', 200, 30)).toBe(true);
    expect(m.due(259)).toBe(false);
    expect(m.due(260)).toBe(true);
  });
});

describe('i18n keys', () => {
  it.each(['ui.officer.tool.needOfficer', 'ui.officer.tool.needSquad', 'ui.officer.tool.needSupport'])('%s', (k) => {
    expect(k in en && k in zh).toBe(true);
  });
});
