import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { roleAvailable, ROLE_ERROR_KEY, spawnSelectable, defaultSpawn, draftReady, deployUiNext, countdownS, recommendAfterReject, deployKey, type RoleDef, type RoleCtx, type SpawnPoint } from './deploy-validate';
import en from '../ui/i18n/en.json';
import zh from '../ui/i18n/zh-Hans.json';

const load = (id: string): RoleDef => {
  const j = JSON.parse(readFileSync(new URL(`../../../data/roles/${id}.json`, import.meta.url).pathname, 'utf8'));
  return { id: j.id, maxPerSquad: j.max_per_squad ?? null, maxPerTeam: j.max_per_team ?? null, unlock: j.unlock ? { squadSizeMin: j.unlock.squad_size_min, requiresSquadLeader: j.unlock.requires_squad_leader } : null };
};
const ctx = (o: Partial<RoleCtx> = {}): RoleCtx => ({ squadKind: 'infantry', squadSize: 4, isLeader: false, inSquadOfRole: 0, inTeamOfRole: 0, crewmanCap: 3, ...o });

describe('role availability (real /data/roles)', () => {
  const [rifle, medic, support, officer, crew] = ['rifleman', 'medic', 'support', 'officer', 'crewman'].map(load) as [RoleDef, RoleDef, RoleDef, RoleDef, RoleDef];
  it('unassigned is rifleman only', () => {
    expect(roleAvailable(rifle, ctx({ squadKind: null }))).toBeNull();
    expect(roleAvailable(medic, ctx({ squadKind: null }))).toBe('unassigned');
  });
  it('officer needs leader and squad ≥ 2; support needs ≥ 3; medic ≤ 2 per squad', () => {
    expect(roleAvailable(officer, ctx({ squadSize: 1, isLeader: true }))).toBe('squadTooSmall');
    expect(roleAvailable(officer, ctx({ squadSize: 2, isLeader: false }))).toBe('leaderOnly');
    expect(roleAvailable(officer, ctx({ squadSize: 2, isLeader: true }))).toBeNull();
    expect(roleAvailable(support, ctx({ squadSize: 2 }))).toBe('squadTooSmall');
    expect(roleAvailable(support, ctx({ squadSize: 3, inSquadOfRole: 1 }))).toBe('full');
    expect(roleAvailable(medic, ctx({ inSquadOfRole: 2 }))).toBe('full');
    expect(roleAvailable(medic, ctx({ inTeamOfRole: 12 }))).toBe('teamFull');
  });
  it('armor crew takes crewman only; infantry cannot', () => {
    expect(roleAvailable(crew, ctx({ squadKind: 'armor_crew' }))).toBeNull();
    expect(roleAvailable(crew, ctx({ squadKind: 'armor_crew', inSquadOfRole: 3 }))).toBe('full');
    expect(roleAvailable(rifle, ctx({ squadKind: 'armor_crew' }))).toBe('wrongSquadKind');
    expect(roleAvailable(crew, ctx())).toBe('notInArmor');
    expect(roleAvailable(medic, ctx({ squadKind: 'recon' }))).toBe('wrongSquadKind');
  });
  it('every error has en and zh keys', () => {
    for (const k of Object.values(ROLE_ERROR_KEY)) {
      expect(k in en).toBe(true);
      expect(k in zh).toBe(true);
    }
  });
});

const P = (id: number, kind: 'hq' | 'garrison', status: SpawnPoint['status'] = 'available'): SpawnPoint => ({ id, kind, status });

describe('spawn points', () => {
  it('blocked points are disabled with a reason key', () => {
    expect(spawnSelectable(P(1, 'garrison', 'blocked'), false)).toEqual({ ok: false, reasonKey: 'ui.deploy.spawn.blocked' });
    expect(spawnSelectable(P(1, 'hq'), false).ok).toBe(true);
  });
  it('default prefers available HQ, then any available, then HQ', () => {
    expect(defaultSpawn([P(1, 'garrison'), P(2, 'hq')])).toBe(2);
    expect(defaultSpawn([P(1, 'garrison'), P(2, 'hq', 'cooldown')])).toBe(1);
    expect(defaultSpawn([P(1, 'garrison', 'blocked'), P(2, 'hq', 'cooldown')])).toBe(2);
  });
  it('after a rejection the recommendation skips the refused point', () => {
    expect(recommendAfterReject([P(1, 'hq'), P(2, 'garrison')], 1)).toBe(2);
    expect(recommendAfterReject([P(1, 'hq')], 1)).toBe(1);
  });
  it('spawn reason keys exist in both languages', () => {
    for (const s of ['blocked', 'cooldown', 'destroyed', 'full', 'locked']) {
      expect(`ui.deploy.spawn.${s}` in en && `ui.deploy.spawn.${s}` in zh).toBe(true);
    }
  });
});

describe('deploy UI machine', () => {
  it('draft → ready → requesting → waiting → deployed', () => {
    let s = deployUiNext('browsing', { t: 'draft', ready: true });
    expect(s).toBe('ready');
    s = deployUiNext(s, { t: 'request' });
    expect(s).toBe('requesting');
    s = deployUiNext(s, { t: 'accepted', spawnInS: 8 });
    expect(s).toBe('waiting');
    expect(deployUiNext(s, { t: 'deployed' })).toBe('deployed');
  });
  it('duplicate requests and unstable links do not send; rejection returns to browsing without re-requesting', () => {
    expect(deployUiNext('requesting', { t: 'request' })).toBe('requesting');
    expect(deployUiNext('ready', { t: 'request' }, true)).toBe('ready');
    const r = deployUiNext('requesting', { t: 'rejected' });
    expect(r).toBe('rejected');
    expect(deployUiNext(r, { t: 'request' })).toBe('rejected');
    expect(deployUiNext(r, { t: 'ack' })).toBe('browsing');
    expect(deployUiNext('requesting', { t: 'accepted', spawnInS: 0 })).toBe('deployed');
  });
  it('draft readiness and countdown', () => {
    expect(draftReady({ spawnId: 1, role: 'rifleman', loadout: 0 })).toBe(true);
    expect(draftReady({ spawnId: 1, role: 'rifleman' })).toBe(false);
    expect(countdownS(1000, 880, 60)).toBe(2);
    expect(countdownS(1000, 941, 60)).toBe(1);
    expect(countdownS(1000, 1200, 60)).toBe(0);
  });
  it('keyboard map', () => {
    expect(deployKey('3')).toEqual({ kind: 'spawn', index: 2 });
    expect(deployKey('Enter')).toEqual({ kind: 'deploy' });
    expect(deployKey('q')).toEqual({ kind: 'role', dir: -1 });
    expect(deployKey('Escape')).toEqual({ kind: 'pause' });
    expect(deployKey('0')).toBeUndefined();
  });
});
