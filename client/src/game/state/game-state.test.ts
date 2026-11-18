import { describe, expect, it } from 'vitest';
import { EventBus, type GameEvents } from '../event-bus';
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
    gs.apply({ type: 'score', player: 1, kind: 0, value: 10 });
    gs.apply({ type: 'score', player: 1, kind: 0, value: -3 });
    gs.apply({ type: 'objective', objective: 4, owner: 1, reason: 0 });
    gs.apply({ type: 'spawnpoint', id: 9, kind: 0, team: 1, pos: [10, 20], state: 1 });
    gs.apply({ type: 'roster', player: 1, nick: 'a', team: 0, squad: 1, role: 0, flags: 0, seq: 1 });
    gs.apply({ type: 'notice', code: 7, args: [1, 2, 3] });
    gs.apply({ type: 'phase', phase: 2, endsAt: 900 });
    expect(seen).toEqual(['death', 'revive', 'objective.changed', 'spawnpoint.changed', 'roster.changed', 'notice', 'match.phase']);
    expect(gs.scores.get(1)).toBe(7);
    expect(gs.objectives.get(4)).toEqual({ owner: 1, reason: 0 });
    expect(gs.spawnPoints.get(9)?.pos).toEqual([10, 20]);
    expect(gs.roster.get(1)?.name).toBe('a');
    expect(gs.matchPhase).toEqual({ phase: 2, endsAt: 900 });
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
