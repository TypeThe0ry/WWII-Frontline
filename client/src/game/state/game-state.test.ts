import { describe, expect, it } from 'vitest';
import { EventBus, type GameEvents } from '../event-bus';
import { ScoreCategory, TeamBalanceCode, TeamMoveFlags } from './c4';
import { GameState } from './game-state';

function make() {
  const bus = new EventBus<GameEvents>();
  const seen: string[] = [];
  for (const k of ['kill.feed', 'objective.changed', 'roster.changed', 'match.phase', 'deploy.state', 'notice', 'death', 'revive', 'spawnpoint.changed'] as const) {
    bus.on(k, () => seen.push(k));
  }
  return { bus, seen, gs: new GameState(bus) };
}

describe('GameState.apply', () => {
  it('kill -> kill feed + bus event, distance in metres', () => {
    const { gs, seen } = make();
    gs.apply({ type: 'kill', victim: 2, killer: 1, weapon: 3, headshot: true, distQ: 400 });
    expect(seen).toEqual(['kill.feed']);
    expect(gs.killFeed.at(0)).toMatchObject({ killer: 1, victim: 2, headshot: true, dist: 100 });
    expect(gs.version.value).toBe(1);
  });

  it('maps every event type to state or bus', () => {
    const { gs, seen } = make();
    gs.apply({ type: 'death', victim: 2, respawnAfter: 5, canBeRevived: true });
    gs.apply({ type: 'revive', medic: 1, target: 2 });
    gs.apply({ type: 'score', player: 1, kind: 0, category: ScoreCategory.Combat, reason: 0, value: 10 });
    gs.apply({ type: 'score', player: 1, kind: 0xc0, category: ScoreCategory.Support, reason: 0, value: -3 });
    gs.apply({ type: 'objective', objective: 4, owner: 1, reason: 0 });
    gs.apply({ type: 'spawnpoint', id: 9, kind: 0, team: 1, pos: [10, 20], state: 1 });
    gs.apply({ type: 'roster', player: 1, nick: 'a', team: 0, squad: 1, role: 0, flags: 0, seq: 1 });
    gs.apply({ type: 'notice', code: 7, args: [1, 2, 3] });
    gs.apply({ type: 'phase', phase: 2, endsAt: 900 });
    expect(seen).toEqual(['death', 'revive', 'objective.changed', 'spawnpoint.changed', 'roster.changed', 'notice', 'match.phase']);
    expect(gs.scores.get(1)).toBe(7);
    expect(gs.scoresByCategory.get(1)).toEqual([10, 0, 0, -3]);
    expect(gs.objectives.get(4)).toEqual({ owner: 1, reason: 0 });
    expect(gs.spawnPoints.get(9)?.pos).toEqual([10, 20]);
    expect(gs.roster.get(1)?.name).toBe('a');
    expect(gs.matchPhase).toEqual({ phase: 2, endsAt: 900 });
  });

  it('does not carry score totals into a reused roster slot', () => {
    const { gs } = make();
    const score = { type: 'score', player: 1, kind: 0x41, category: ScoreCategory.Offense, reason: 1, value: 50 } as const;
    gs.apply(score);
    gs.apply({ type: 'roster', player: 1, nick: 'p', team: 0, squad: 0, role: 0, flags: 0, seq: 2 });
    const left = { type: 'roster', player: 1, nick: '', team: 0, squad: 0, role: 0, flags: 0, left: true } as const;
    gs.apply({ ...left, seq: 1 });
    expect(gs.scores.get(1)).toBe(50);
    gs.apply({ ...left, seq: 3 });
    expect(gs.scores.has(1)).toBe(false);
    expect(gs.scoresByCategory.has(1)).toBe(false);
    gs.apply(score);
    expect(gs.scoresByCategory.get(1)).toEqual([0, 50, 0, 0]);
  });

  it('stale roster delta does not emit; leaving removes', () => {
    const { gs, seen } = make();
    gs.apply({ type: 'roster', player: 1, nick: 'a', team: 0, squad: 1, role: 0, flags: 0, seq: 5 });
    gs.apply({ type: 'roster', player: 1, nick: 'old', team: 0, squad: 1, role: 0, flags: 0, seq: 2 });
    expect(seen).toEqual(['roster.changed']);
    gs.apply({ type: 'roster', player: 1, nick: '', team: 0, squad: 0, role: 0, flags: 0, seq: 6, left: true });
    expect(gs.roster.size).toBe(0);
  });

  it('deploy transitions emit deploy.state; illegal ones do not', () => {
    const { gs, seen } = make();
    expect(gs.applyDeploy({ type: 'request' })).toBe(false);
    expect(gs.applyDeploy({ type: 'open' })).toBe(true);
    expect(seen).toEqual(['deploy.state']);
  });

  it('switches only on valid Moved, resets pending deploy, and accepts only the new team projection', () => {
    const { gs, bus } = make();
    gs.seedTeam(0);
    const spawn = { type: 'spawnpoint', id: 9, kind: 0, team: 0, pos: [10, 20], state: 1 } as const;
    gs.apply(spawn);
    gs.apply({ type: 'objective', objective: 3, owner: 0, reason: 0 });
    gs.applyDeploy({ type: 'open' });
    gs.applyDeploy({ type: 'request' });
    gs.apply({ type: 'teamBalanceNotice', code: TeamBalanceCode.Invite, args: [1, 20] });
    gs.apply({ type: 'teamBalanceNotice', code: TeamBalanceCode.Moved, args: [9, 0] });
    expect(gs.team).toBe(0);
    expect(gs.deploy.phase).toBe('Requested');

    const moves: GameEvents['team.changed'][] = [];
    bus.on('team.changed', (move) => {
      expect(gs.spawnPoints.size).toBe(0);
      expect(gs.deploy.phase).toBe('Idle');
      moves.push(move);
    });
    const move = { type: 'teamBalanceNotice', code: TeamBalanceCode.Moved, args: [1 | (TeamMoveFlags.VolunteerBonus << 8), 40] } as const;
    gs.apply(move);
    expect(gs.applyDeployResult({ kind: 'deployResult', ok: true, spawnTick: 123, reason: 0 })).toBe(false);
    gs.apply(spawn);
    gs.apply({ ...spawn, id: 10, team: 1 });
    gs.apply(move); // Duplicate must not clear a projection rebuilt since the move.
    expect([...gs.spawnPoints.keys()]).toEqual([10]);
    expect(gs.objectives.has(3)).toBe(true);
    expect(moves).toEqual([{ team: 1, flags: TeamMoveFlags.VolunteerBonus, tick: 40 }]);
  });

  /** reconnect_rebuild: Welcome + full roster reproduces the pre-disconnect roster. */
  it('reconnect rebuilds roster from a full list', () => {
    const { gs } = make();
    for (let i = 1; i <= 5; i++) gs.apply({ type: 'roster', player: i, nick: `p${i}`, team: i % 2, squad: 1, role: 0, flags: 0, seq: i });
    const before = gs.roster.list();
    gs.deploy.apply({ type: 'reset' });
    gs.roster.replaceAll(before, 100);
    expect(gs.roster.list()).toEqual(before);
  });

  it('kill feed reuses its slot objects (no per-kill allocation)', () => {
    const { gs } = make();
    const ev = { type: 'kill', victim: 2, killer: 1, weapon: 3, headshot: false, distQ: 40 } as const;
    for (let i = 0; i < 8; i++) gs.apply(ev);
    const held = Array.from({ length: 8 }, (_, i) => gs.killFeed.at(i));
    for (let i = 0; i < 1000; i++) gs.apply(ev);
    const after = new Set(Array.from({ length: 8 }, (_, i) => gs.killFeed.at(i)));
    expect(after.size).toBe(8);
    for (const h of held) expect(after.has(h)).toBe(true);
  });
});

describe('team resources', () => {
  it('keeps the latest team pool authoritative across C4 deltas and team moves', () => {
    const { gs, bus } = make();
    const seen: number[] = [];
    bus.on('resource.delta', ({ value }) => seen.push(value));
    gs.seedTeam(0);
    const income = { tick: 1800, team: 0, timer: 60, manpower: 242, munitions: 98, fuel: 50, frontLinePos: 0, objectives: [] };
    gs.applyWorldState(income);
    gs.apply({ type: 'resourceDelta', team: 0, kind: 0, value: 42, reason: 0 });
    gs.apply({ type: 'resourceDelta', team: 1, kind: 0, value: 42, reason: 0 });
    gs.applyWorldState({ ...income, tick: 1770, manpower: 200 });
    gs.applyWorldState({ ...income, tick: 1830, team: 1, manpower: 400 });
    expect(seen).toEqual([42]);
    expect(gs.worldState).toEqual(income);

    gs.apply({ type: 'teamBalanceNotice', code: TeamBalanceCode.Moved, args: [1, 1830] });
    gs.applyWorldState({ ...income, tick: 1800, team: 1 });
    gs.applyWorldState({ ...income, tick: 1860, team: 0 });
    expect(gs.worldState).toBeUndefined();
    gs.applyWorldState({ ...income, tick: 1860, team: 1, manpower: 400 });
    expect(gs.worldState?.manpower).toBe(400);
  });
});
