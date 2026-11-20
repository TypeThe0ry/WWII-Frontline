/**
 * Client game state fed by C4 reliable events and C2 world state. Consumers read the typed
 * accessors and subscribe to the bus, or watch the `version` signal.
 * `game` imports nothing from render / ui / audio (ARCHITECTURE 4.3.9).
 */
import { signal, type ReadonlySignal } from '@preact/signals';
import type { DeployResultMsg } from '../../net/codec';
import type { WorldState } from '../../net/connect';
import type { EventBus, GameEvents } from '../event-bus';
import { DIST_Q_M, ROSTER_ALIVE, TeamBalanceCode, type C4Event } from './c4';
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
  readonly structures = new Map<number, Extract<C4Event, { type: 'structure' }>>();
  readonly spawnPoints = new Map<number, SpawnPointInfo>();
  /** Deltas observed since joining, not a replay of earlier scoring. */
  readonly scores = new Map<number, number>();
  /** Category order matches C4 ScoreCategory: Combat, Offense, Defense, Support. */
  readonly scoresByCategory = new Map<number, [number, number, number, number]>();
  matchPhase = { phase: 0, endsAt: 0 };
  private teamValue: number | undefined;
  private worldStateValue: WorldState | undefined;
  private worldStateFloor: number | undefined;

  /** Absolute resource pools from C2. C4 deltas are notifications on an independent stream. */
  get worldState(): WorldState | undefined {
    return this.worldStateValue;
  }

  applyWorldState(state: WorldState): void {
    if (state.team !== this.teamValue) return;
    if (this.worldStateFloor !== undefined && ((state.tick - this.worldStateFloor) | 0) <= 0) return;
    this.worldStateFloor = state.tick;
    this.worldStateValue = state;
    this.versionSig.value++;
  }

  get team(): number | undefined {
    return this.teamValue;
  }

  private readonly versionSig = signal(0);
  /** Bumped after every applied event; UI effects subscribe to this. */
  readonly version: ReadonlySignal<number> = this.versionSig;

  constructor(private readonly bus: EventBus<GameEvents>) {}

  /** Welcome seeds the team before the reliable event stream is applied. */
  seedTeam(team: number): void {
    this.teamValue = team;
  }

  /** An unsolicited/duplicate result cannot change the FSM outside Requested. */
  applyDeployResult(result: DeployResultMsg): boolean {
    return this.applyDeploy(result.ok
      ? { type: 'result', ok: true, spawnTick: result.spawnTick }
      : { type: 'result', ok: false, reason: `deploy.reject.${DEPLOY_REJECT_REASONS[result.reason] ?? 'unknown'}` });
  }

  /** Deploy transitions come from C0 / `self_state`, or a C4 team move reset. */
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
      case 'score': {
        this.scores.set(ev.player, (this.scores.get(ev.player) ?? 0) + ev.value);
        const categories = this.scoresByCategory.get(ev.player) ?? [0, 0, 0, 0];
        categories[ev.category] += ev.value;
        this.scoresByCategory.set(ev.player, categories);
        break;
      }
      case 'resourceDelta':
        if (ev.team !== this.teamValue) return;
        this.bus.emit('resource.delta', { team: ev.team, kind: ev.kind, value: ev.value, reason: ev.reason });
        break;
      case 'objective':
        this.objectives.set(ev.objective, { owner: ev.owner, reason: ev.reason });
        this.bus.emit('objective.changed', { objective: ev.objective, owner: ev.owner, reason: ev.reason });
        break;
      case 'spawnpoint':
        if (this.teamValue !== undefined && ev.team !== this.teamValue) return;
        if (ev.kind === 1 && ev.state === 2) this.spawnPoints.delete(ev.id);
        else this.spawnPoints.set(ev.id, { id: ev.id, kind: ev.kind, team: ev.team, pos: ev.pos, state: ev.state });
        this.bus.emit('spawnpoint.changed', { id: ev.id, team: ev.team, state: ev.state });
        break;
      case 'structure':
        if (ev.state === 2) this.structures.delete(ev.id);
        else this.structures.set(ev.id, ev);
        break;
      case 'roster': {
        const changed = this.roster.apply({
          id: ev.player,
          seq: ev.seq,
          entry: ev.left ? undefined : { team: ev.team, squad: ev.squad, role: ev.role, name: ev.nick },
        });
        if (changed) {
          if (ev.left) {
            this.scores.delete(ev.player);
            this.scoresByCategory.delete(ev.player);
          }
          this.bus.emit('roster.changed', { player: ev.player, joined: !ev.left });
          if (!ev.left && (ev.flags & ROSTER_ALIVE) !== 0) this.bus.emit('player.alive', { player: ev.player });
        }
        break;
      }
      case 'notice':
        this.bus.emit('notice', { code: ev.code, args: ev.args });
        break;
      case 'phase':
        this.matchPhase = { phase: ev.phase, endsAt: ev.endsAt };
        this.bus.emit('match.phase', { phase: ev.phase, endsAt: ev.endsAt });
        break;
      case 'teamBalanceNotice': {
        if (ev.code !== TeamBalanceCode.Moved) return;
        const [sideAndFlags, tick] = ev.args;
        const team = sideAndFlags & 0xff;
        const flags = sideAndFlags >>> 8;
        if ((team !== 0 && team !== 1) || team === this.teamValue) return;
        this.teamValue = team;
        this.spawnPoints.clear();
        this.structures.clear();
        this.worldStateValue = undefined;
        this.worldStateFloor = tick >>> 0;
        // Match-wide roster, objectives, scores and kill feed survive a team change.
        this.deploy.apply({ type: 'reset' });
        this.bus.emit('team.changed', { team, flags, tick: tick >>> 0 });
        this.bus.emit('deploy.state', { phase: this.deploy.phase });
        break;
      }
      default:
        return; // Other server-rule events are exposed by the C4 wire callback.
    }
    this.versionSig.value++;
  }
}
