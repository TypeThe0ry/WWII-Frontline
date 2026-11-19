import { describe, expect, it } from 'vitest';
import { HQ_SPAWN_ID } from '../../game/deploy-actions';
import type { MapSpawn } from '../map';
import { makeDeployRequest, pickDefaultSpawn, recommendSpawn } from './model';

const hq = { id: HQ_SPAWN_ID, label: 'HQ', kind: 'hq', status: 'available', x: 1024, z: 256 } satisfies MapSpawn;
const garrison = { ...hq, id: 12, kind: 'garrison', label: 'Farm' } satisfies MapSpawn;
const rifleman = { id: 'rifleman', index: 3, label: 'Rifleman' };
const loadout = { index: 12, roleId: 'rifleman', label: 'Standard issue' };
const points = [garrison, hq];

describe('Deploy selection', () => {
  it('uses actual wire indices and differentiates unknown from Unassigned', () => {
    expect(makeDeployRequest(hq, rifleman, loadout, undefined)).toBeUndefined();
    expect(makeDeployRequest(hq, rifleman, loadout, null)).toEqual({ spawnId: HQ_SPAWN_ID, role: 3, loadout: 12, squadId: null });
    expect(makeDeployRequest(garrison, rifleman, loadout, 4)).toMatchObject({ spawnId: garrison.id, squadId: 4 });
  });
  it('blocks unavailable selections and the real server’s HQ-only Unassigned rule', () => {
    expect(makeDeployRequest(garrison, rifleman, loadout, null)).toBeUndefined();
    expect(makeDeployRequest(hq, { ...rifleman, unavailableReason: 'Full' }, loadout, 4)).toBeUndefined();
    expect(makeDeployRequest(hq, rifleman, { ...loadout, roleId: 'medic' }, 4)).toBeUndefined();
    expect(makeDeployRequest(hq, rifleman, { ...loadout, unavailableReason: 'Locked' }, 4)).toBeUndefined();
    expect(makeDeployRequest({ ...hq, status: 'blocked' }, rifleman, loadout, null)).toBeUndefined();
  });
  it('defaults to the HQ and recommends alternatives without offering invalid Unassigned spawns', () => {
    expect(pickDefaultSpawn(points, null)).toBe(HQ_SPAWN_ID);
    expect(pickDefaultSpawn([garrison], null)).toBeUndefined();
    expect(recommendSpawn(points, HQ_SPAWN_ID, 4)).toBe(garrison.id);
    expect(recommendSpawn(points, HQ_SPAWN_ID, null)).toBeUndefined();
  });
});
