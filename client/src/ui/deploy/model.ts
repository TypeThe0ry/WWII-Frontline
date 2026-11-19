import { HQ_SPAWN_ID } from '../../game/deploy-actions';
import type { DeployRequest } from '../../game/deploy-actions';
import { defaultSpawn } from '../../game/deploy-validate';
import type { SquadKind } from '../../game/deploy-validate';
import type { MapSpawn } from '../map';

export interface DeploySquad {
  id: number;
  label: string;
  kind?: SquadKind;
  members?: number;
  capacity?: number;
  locked?: boolean;
  isLeader?: boolean;
}
export interface DeployRole {
  index: number;
  id: string;
  label: string;
  unavailableReason?: string;
  quota?: { used: number; limit: number };
}
export interface DeployItem {
  slot: string;
  label: string;
  magazine?: number;
  rpm?: number;
  weightKg?: number;
}
export interface DeployLoadout {
  index: number;
  roleId: string;
  label: string;
  unavailableReason?: string;
  items?: readonly DeployItem[];
}

const SPAWN_STATUS: Record<Exclude<MapSpawn['status'], 'available'>, string> = {
  blocked: 'Blocked', cooldown: 'On cooldown', destroyed: 'Destroyed', full: 'Full', locked: 'Locked',
};

export function spawnIssue(spawn: MapSpawn, squadId: number | null | undefined): string | undefined {
  // Unlike the early UI helper, actual Deploy validation is HQ-only when Unassigned.
  if (squadId === null && spawn.kind !== 'hq') return 'Unassigned players must deploy at HQ.';
  if (!wireIndex(spawn.id, 0xffff) || (spawn.kind === 'hq') !== (spawn.id === HQ_SPAWN_ID)) return 'Invalid spawn point.';
  return spawn.status !== 'available' ? SPAWN_STATUS[spawn.status] : undefined;
}

export function pickDefaultSpawn(points: readonly MapSpawn[], squadId: number | null | undefined): number | undefined {
  return defaultSpawn(points.filter((point) => !spawnIssue(point, squadId)))
    ?? points.find((point) => point.id === HQ_SPAWN_ID && point.kind === 'hq')?.id;
}

export function recommendSpawn(points: readonly MapSpawn[], refused: number, squadId: number | null | undefined): number | undefined {
  return points.find((point) => point.id !== refused && !spawnIssue(point, squadId))?.id;
}

function wireIndex(index: number, max: number, min = 0): boolean {
  return Number.isInteger(index) && index >= min && index <= max;
}

export function makeDeployRequest(spawn: MapSpawn | undefined, role: DeployRole | undefined, loadout: DeployLoadout | undefined,
  squadId: number | null | undefined): DeployRequest | undefined {
  if (squadId === undefined || (squadId !== null && !wireIndex(squadId, 255, 1))) return undefined;
  if (!spawn || spawnIssue(spawn, squadId)) return undefined;
  if (!role || !wireIndex(role.index, 255) || role.unavailableReason || (squadId === null && role.id !== 'rifleman')) return undefined;
  if (!loadout || !wireIndex(loadout.index, 255) || loadout.roleId !== role.id || loadout.unavailableReason) return undefined;
  return { spawnId: spawn.id, role: role.index, loadout: loadout.index, squadId };
}
