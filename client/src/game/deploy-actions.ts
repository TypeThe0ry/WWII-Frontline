import type { Transport } from '../net/transport';
import type { SimWasmModule, WasmServerC0 } from '../net/wasm/types';
import type { SquadKind } from './deploy-validate';

// C0 payloads, without transport framing. Layouts: crates/protocol/src/{channel.rs,msg/c0.rs}.
const DEPLOY_REQUEST = 0x10;
const DEPLOY_RESULT = 0x11;
const SQUAD_OP = 0x12;
export const HQ_SPAWN_ID = 0xffff;
export const UNASSIGNED_SQUAD_ID = 0;

export interface DeployRequest {
  spawnId: number;
  /** Index into the server's GameData.roles, not an index in the filtered UI list. */
  role: number;
  /** Index into the server's GameData.loadouts, not a weapon ID. */
  loadout: number;
  /** Null requests the wire's Unassigned sentinel. Membership must already be authoritative. */
  squadId: number | null;
}

export type DeployResult =
  | { ok: true; spawnTick: number }
  | { ok: false; reason: number; reasonKey: string; message: string };

// Numeric order is DeployError::code in crates/sim/src/deploy/validate.rs.
const reasons = [
  null,
  ['notOnRoster', 'Player is not on the roster.'],
  ['notDeployable', 'You cannot deploy while alive or downed.'],
  ['wrongSquad', 'Your squad membership has changed.'],
  ['unknownRole', 'Unknown role.'],
  ['roleFull', 'That role is full in your squad.'],
  ['roleTeamFull', 'That role is full on your team.'],
  ['squadTooSmall', 'Your squad is too small for that role.'],
  ['leaderOnly', 'Only the squad leader can take that role.'],
  ['notInArmor', 'Crewmen serve in armor crews only.'],
  ['wrongSquadKind', 'That role is unavailable in this squad type.'],
  ['unassignedRole', 'Unassigned players must deploy as Rifleman.'],
  ['unknownLoadout', 'Unknown loadout.'],
  ['loadoutWrongRole', 'That loadout belongs to another role.'],
  ['loadoutWrongFaction', 'That loadout belongs to another faction.'],
  ['loadoutLocked', 'That loadout is locked.'],
  ['unassignedNeedsHq', 'Unassigned players must deploy at HQ.'],
  ['noHq', 'Your team has no HQ spawn.'],
  ['noSuchPoint', 'That spawn point no longer exists.'],
  ['pointWrongTeam', 'That spawn point is not on your team.'],
  ['pointNotActive', 'That spawn point is not active.'],
  ['pointBlocked', 'That spawn point is blocked.'],
  ['pointNoSlots', 'That spawn point has no slots left.'],
  ['roleCooldown', 'You cannot change roles yet.'],
] as const;

type DeployCodec = Pick<SimWasmModule, 'encodeC0'> & {
  decodeServerC0(bytes: Uint8Array): Pick<WasmServerC0, 'kind' | 'reason' | 'ok' | 'spawnTick' | 'free'>;
};

export function decodeDeployResult(bytes: Uint8Array, codec: Pick<DeployCodec, 'decodeServerC0'>): DeployResult | null {
  if (bytes[0] !== DEPLOY_RESULT) return null;
  const result = codec.decodeServerC0(bytes);
  try {
    if (result.kind !== DEPLOY_RESULT) return null;
    if (result.ok) return { ok: true, spawnTick: result.spawnTick };
    const reason = result.reason;
    const detail = reasons[reason];
    return { ok: false, reason, reasonKey: `ui.deploy.err.${detail?.[0] ?? 'unknown'}`, message: detail?.[1] ?? `Deploy rejected (code ${reason}).` };
  } finally {
    result.free();
  }
}

function uint(value: number, max: number, name: string, min = 0): number {
  if (!Number.isInteger(value) || value < min || value > max) throw new RangeError(`Invalid ${name}: ${value}`);
  return value;
}

function deployPayload(req: DeployRequest): Uint8Array {
  const spawn = uint(req.spawnId, 0xffff, 'spawn point');
  return Uint8Array.of(spawn >> 8, spawn & 0xff, uint(req.role, 255, 'role'),
    uint(req.loadout, 255, 'loadout'), req.squadId === null ? UNASSIGNED_SQUAD_ID : uint(req.squadId, 255, 'squad', 1));
}

export const SQUAD_NAMES = {
  infantry: ['Alpha', 'Bravo', 'Charlie', 'Delta', 'Echo', 'Foxtrot'],
  armor_crew: ['Armor-1', 'Armor-2', 'Armor-3'],
  recon: ['Recon-1', 'Recon-2'],
} as const satisfies Record<SquadKind, readonly string[]>;
const squadKinds: Record<SquadKind, number> = { infantry: 0, armor_crew: 1, recon: 2 };

function squadPayload(op: number, squad = 0, nameKey = ''): Uint8Array {
  // Only the preset names above are sent, all within the wire's 32-byte and sim's 16-character limits.
  const name = new TextEncoder().encode(nameKey);
  return Uint8Array.of(op, squad, 0, name.length, ...name);
}

export interface DeployActions {
  deploy(req: DeployRequest): Promise<DeployResult>;
  /** Squad commands return after dispatch only. Confirm changes from the authoritative roster/squad data. */
  createSquad(kind: SquadKind, nameKey: string): void;
  joinSquad(squadId: number): void;
  leaveSquad(): void;
  /** The server applies lock/unlock to the sender's current squad; no target is accepted. */
  lockSquad(locked: boolean): void;
  dispose(): void;
}

type DeployTransport = Pick<Transport, 'sendC0' | 'onC0' | 'onClose'>;

/** One adapter per connected transport; it must be the only sender of DeployRequest on it. */
export function createDeployActions(transport: DeployTransport, codec: DeployCodec, timeoutMs = 8000): DeployActions {
  let pending: { resolve: (result: DeployResult) => void; reject: (e: Error) => void; timer: ReturnType<typeof setTimeout> } | undefined;
  let closed = false;
  let ambiguous = false;
  const clearPending = () => {
    const request = pending;
    pending = undefined;
    if (request) clearTimeout(request.timer);
    return request;
  };
  const offC0 = transport.onC0((bytes) => {
    if (!pending) return;
    try {
      const result = decodeDeployResult(bytes, codec);
      if (result) clearPending()?.resolve(result);
    } catch {
      // Ignore malformed C0. The outstanding request will time out unless a valid result arrives.
    }
  });
  const offClose = transport.onClose(() => dispose());

  function dispose() {
    if (closed) return;
    closed = true;
    offC0();
    offClose();
    clearPending()?.reject(new Error('The deploy connection closed.'));
  }
  function send(bytes: Uint8Array) {
    if (closed) throw new Error('The deploy connection closed.');
    transport.sendC0(bytes);
  }

  function squad(op: number, id?: number, name?: string) {
    send(codec.encodeC0(SQUAD_OP, squadPayload(op, id, name)));
  }

  return {
    async deploy(req) {
      if (closed) throw new Error('The deploy connection closed.');
      if (pending) throw new Error('A deploy request is already pending.');
      if (ambiguous) throw new Error('Deploy outcome is unknown. Reconnect before retrying.');
      const bytes = codec.encodeC0(DEPLOY_REQUEST, deployPayload(req));
      return new Promise((resolve, reject) => {
        const fail = () => {
          // No request ID on DeployResult: a late reply could be mistaken for the next request.
          ambiguous = true;
          clearPending()?.reject(new Error('No deploy result received. Reconnect before retrying.'));
        };
        pending = { resolve, reject, timer: setTimeout(fail, timeoutMs) };
        try { send(bytes); } catch { if (pending) fail(); }
      });
    },
    createSquad(kind, nameKey) {
      const names: readonly string[] = SQUAD_NAMES[kind];
      if (!names?.includes(nameKey)) throw new RangeError('Unknown preset squad name.');
      squad(1, squadKinds[kind], nameKey);
    },
    joinSquad(id) { squad(2, uint(id, 255, 'squad', 1)); },
    leaveSquad() { squad(3); },
    lockSquad(locked) { squad(locked ? 7 : 8); },
    dispose,
  };
}
