/**
 * Client game state fed by C4 reliable events (M1-040). `apply` is the only writer; consumers
 * read the typed accessors and subscribe to the bus, or watch the per-domain `version` signals.
 * `game` imports nothing from render / ui / audio (ARCHITECTURE 4.3.9).
 */
import { signal, type ReadonlySignal } from '@preact/signals';
import type { DeployResultMsg } from '../../net/codec';
import type { EventBus, GameEvents } from '../event-bus';
import { DIST_Q_M, type C4Event } from './c4';
import { DeployFsm, type DeployEvent } from './deploy-fsm';
import { KillFeed } from './kill-feed';
import { Roster } from './roster';

export interface ObjectiveState {
  owner: number;
  reason: number;
}
export interface SpawnPointInfo {
  id: number;
  kind: number;
  team: number;
  pos: readonly [number, number];
  state: number;
}

// Index = fixed protocol reason from sim's DeployError::code(). Index 0 is success.
const DEPLOY_REJECT_REASONS = [
  'unknown', 'not_on_roster', 'not_deployable', 'wrong_squad', 'unknown_role', 'role_full',
  'role_team_full', 'squad_too_small', 'leader_only', 'not_in_armor', 'wrong_squad_kind',
  'unassigned_role', 'unknown_loadout', 'loadout_wrong_role', 'loadout_wrong_faction',
  'loadout_locked', 'unassigned_needs_hq', 'no_hq', 'no_such_point', 'point_wrong_team',
  'point_not_active', 'point_blocked', 'point_no_slots', 'role_cooldown',
] as const;

export class GameState {
  readonly roster = new Roster();
  readonly killFeed = new KillFeed();
  readonly deploy = new DeployFsm();
  readonly objectives = new Map<number, ObjectiveState>();
  readonly spawnPoints = new Map<number, SpawnPointInfo>();
  readonly scores = new Map<number, number>();
  matchPhase = { phase: 0, endsAt: 0 };

  private readonly versionSig = signal(0);
  /** Bumped after every applied event; UI effects subscribe to this. */
  readonly version: ReadonlySignal<number> = this.versionSig;

  constructor(private readonly bus: EventBus<GameEvents>) {}

  /** An unsolicited/duplicate result cannot change the FSM outside Requested. */
  applyDeployResult(result: DeployResultMsg): boolean {
    return this.applyDeploy(result.ok
      ? { type: 'result', ok: true, spawnTick: result.spawnTick }
      : { type: 'result', ok: false, reason: `deploy.reject.${DEPLOY_REJECT_REASONS[result.reason] ?? 'unknown'}` });
  }

  /** Deploy transitions come from C0 / `self_state`, not C4. */
  applyDeploy(ev: DeployEvent): boolean {
    const ok = this.deploy.apply(ev);
    if (ok) {
      const s = this.deploy.state;
      this.bus.emit('deploy.state', { phase: s.phase, spawnTick: s.spawnTick, reason: s.reason });
      this.versionSig.value++;
    }
    return ok;
  }

  /** Applies one reliable server event, updating state and emitting its bus event. */
  apply(ev: C4Event): void {
    switch (ev.type) {
      case 'kill': {
        const dist = ev.distQ * DIST_Q_M;
        this.killFeed.push(ev.killer, ev.victim, ev.weapon, ev.headshot, dist);
        this.bus.emit('kill.feed', { killer: ev.killer, victim: ev.victim, weapon: ev.weapon, headshot: ev.headshot, dist });
        break;
      }
      case 'death':
        this.bus.emit('death', { victim: ev.victim, respawnAfter: ev.respawnAfter, canBeRevived: ev.canBeRevived });
        break;
      case 'revive':
        this.bus.emit('revive', { medic: ev.medic, target: ev.target });
        break;
      case 'score':
        this.scores.set(ev.player, (this.scores.get(ev.player) ?? 0) + ev.value);
        break;
      case 'objective':
        this.objectives.set(ev.objective, { owner: ev.owner, reason: ev.reason });
        this.bus.emit('objective.changed', { objective: ev.objective, owner: ev.owner, reason: ev.reason });
        break;
      case 'spawnpoint':
        this.spawnPoints.set(ev.id, { id: ev.id, kind: ev.kind, team: ev.team, pos: ev.pos, state: ev.state });
        this.bus.emit('spawnpoint.changed', { id: ev.id, team: ev.team, state: ev.state });
        break;
      case 'roster': {
        const changed = this.roster.apply({
          id: ev.player,
          seq: ev.seq,
          entry: ev.left ? undefined : { team: ev.team, squad: ev.squad, role: ev.role, name: ev.nick },
        });
        if (changed) this.bus.emit('roster.changed', { player: ev.player, joined: !ev.left });
        break;
      }
      case 'notice':
        this.bus.emit('notice', { code: ev.code, args: ev.args });
        break;
      case 'phase':
        this.matchPhase = { phase: ev.phase, endsAt: ev.endsAt };
        this.bus.emit('match.phase', { phase: ev.phase, endsAt: ev.endsAt });
        break;
    }
    this.versionSig.value++;
  }
}
