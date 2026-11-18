/** Client pre-validation for the deploy screen (mirror of `crates/sim/src/roles/quota.rs`; the server is authoritative). */
export type SquadKind = 'infantry' | 'armor_crew' | 'recon';
export type RoleError = 'full' | 'teamFull' | 'squadTooSmall' | 'leaderOnly' | 'notInArmor' | 'wrongSquadKind' | 'unassigned';

export interface RoleDef {
  id: string;
  maxPerSquad: number | null;
  maxPerTeam: number | null;
  unlock: { squadSizeMin: number; requiresSquadLeader: boolean } | null;
}
export interface RoleCtx {
  squadKind: SquadKind | null;
  squadSize: number;
  isLeader: boolean;
  inSquadOfRole: number;
  inTeamOfRole: number;
  crewmanCap: number;
  armorRole?: string;
}

export function roleAvailable(def: RoleDef, ctx: RoleCtx): RoleError | null {
  const armor = ctx.armorRole ?? 'crewman';
  if (ctx.squadKind === null) return def.id === 'rifleman' ? null : 'unassigned';
  let squadCap = def.maxPerSquad;
  if (ctx.squadKind === 'armor_crew') {
    if (def.id !== armor) return 'wrongSquadKind';
    squadCap = ctx.crewmanCap;
  } else if (ctx.squadKind === 'recon') {
    if (def.id !== 'rifleman') return 'wrongSquadKind';
  } else if (def.id === armor) return 'notInArmor';
  if (def.unlock) {
    if (def.unlock.requiresSquadLeader && !ctx.isLeader) return 'leaderOnly';
    if (ctx.squadSize < def.unlock.squadSizeMin) return 'squadTooSmall';
  }
  if (squadCap !== null && ctx.inSquadOfRole >= squadCap) return 'full';
  if (def.maxPerTeam !== null && ctx.inTeamOfRole >= def.maxPerTeam) return 'teamFull';
  return null;
}

export const ROLE_ERROR_KEY: Readonly<Record<RoleError, string>> = {
  full: 'ui.deploy.role.full',
  teamFull: 'ui.deploy.role.teamFull',
  squadTooSmall: 'ui.deploy.role.squadTooSmall',
  leaderOnly: 'ui.deploy.role.leaderOnly',
  notInArmor: 'ui.deploy.role.notInArmor',
  wrongSquadKind: 'ui.deploy.role.wrongSquadKind',
  unassigned: 'ui.deploy.role.unassigned',
};

/** Spawn point states (UI_SPEC §4.5.1). */
export type SpawnStatus = 'available' | 'blocked' | 'cooldown' | 'destroyed' | 'full' | 'locked';
export interface SpawnPoint {
  id: number;
  kind: 'hq' | 'garrison';
  status: SpawnStatus;
}
export const SPAWN_REASON_KEY: Readonly<Record<Exclude<SpawnStatus, 'available'>, string>> = {
  blocked: 'ui.deploy.spawn.blocked',
  cooldown: 'ui.deploy.spawn.cooldown',
  destroyed: 'ui.deploy.spawn.destroyed',
  full: 'ui.deploy.spawn.full',
  locked: 'ui.deploy.spawn.locked',
};

export function spawnSelectable(sp: SpawnPoint, unassigned: boolean): { ok: boolean; reasonKey?: string } {
  if (sp.status !== 'available') return { ok: false, reasonKey: SPAWN_REASON_KEY[sp.status] };
  // Unassigned players spawn at HQ / garrison only in M1 — both are the only kinds, so no extra restriction here.
  void unassigned;
  return { ok: true };
}

/** No available point → HQ is auto-selected (GD §2.5). */
export function defaultSpawn(points: readonly SpawnPoint[]): number | undefined {
  return (points.find((p) => p.kind === 'hq' && p.status === 'available') ?? points.find((p) => p.status === 'available') ?? points.find((p) => p.kind === 'hq'))?.id;
}

export interface DeployDraft {
  spawnId?: number;
  role?: string;
  loadout?: number;
  squadId?: number;
}
export const draftReady = (d: DeployDraft): boolean => d.spawnId !== undefined && d.role !== undefined && d.loadout !== undefined;

/** UI state machine (UI_SPEC §4.5.3). A rejection returns to `browsing` and highlights a recommendation; it never re-sends. */
export type DeployUiState = 'browsing' | 'ready' | 'waiting' | 'requesting' | 'deployed' | 'rejected';
export type DeployUiEvent =
  | { t: 'draft'; ready: boolean }
  | { t: 'accepted'; spawnInS: number }
  | { t: 'request' }
  | { t: 'deployed' }
  | { t: 'rejected' }
  | { t: 'ack' };

export function deployUiNext(s: DeployUiState, e: DeployUiEvent, unstable = false): DeployUiState {
  switch (e.t) {
    case 'draft':
      return s === 'browsing' || s === 'ready' ? (e.ready ? 'ready' : 'browsing') : s;
    case 'request':
      // De-duplicate: only one request in flight, none while the link is unstable.
      return s === 'ready' && !unstable ? 'requesting' : s;
    case 'accepted':
      return s === 'requesting' ? (e.spawnInS > 0 ? 'waiting' : 'deployed') : s;
    case 'deployed':
      return s === 'waiting' || s === 'requesting' ? 'deployed' : s;
    case 'rejected':
      return s === 'requesting' ? 'rejected' : s;
    case 'ack':
      return s === 'rejected' ? 'browsing' : s;
  }
}

/** Server-driven countdown: remaining whole seconds until `spawnTick` (ceil), never negative. */
export function countdownS(spawnTick: number, nowTick: number, tickRate: number): number {
  return Math.max(0, Math.ceil((spawnTick - nowTick) / tickRate));
}

/** Rejection → recommended spawn: first available point other than the one just refused. */
export function recommendAfterReject(points: readonly SpawnPoint[], refused: number): number | undefined {
  return points.find((p) => p.id !== refused && p.status === 'available')?.id ?? points.find((p) => p.kind === 'hq')?.id;
}

/** Keyboard (§4.5.4). Returns an action or undefined. */
export type DeployKey = { kind: 'spawn'; index: number } | { kind: 'createSquad' } | { kind: 'lock' } | { kind: 'role'; dir: -1 | 1 } | { kind: 'deploy' } | { kind: 'pause' };
export function deployKey(key: string): DeployKey | undefined {
  if (/^[1-9]$/.test(key)) return { kind: 'spawn', index: Number(key) - 1 };
  switch (key.toLowerCase()) {
    case 'c': return { kind: 'createSquad' };
    case 'l': return { kind: 'lock' };
    case 'q': return { kind: 'role', dir: -1 };
    case 'e': return { kind: 'role', dir: 1 };
    case 'enter': return { kind: 'deploy' };
    case 'escape': return { kind: 'pause' };
    default: return undefined;
  }
}
