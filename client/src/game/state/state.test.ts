import { describe, expect, it } from 'vitest';
import { DeployFsm, type DeployEvent, type DeployPhase } from './deploy-fsm';
import { EventDedup } from './dedup';
import { KillFeed } from './kill-feed';
import { Roster, type RosterDelta } from './roster';

describe('DeployFsm', () => {
  const run = (f: DeployFsm, evs: DeployEvent[]) => evs.map((e) => f.apply(e));

  it('walks the happy path and the death loop', () => {
    const f = new DeployFsm();
    const ok = run(f, [
      { type: 'open' },
      { type: 'request' },
      { type: 'result', ok: true, spawnTick: 300 },
    ]);
    expect(ok).toEqual([true, true, true]);
    expect(f.state).toEqual({ phase: 'Counting', spawnTick: 300, reason: undefined });
    run(f, [{ type: 'spawned' }, { type: 'downed' }, { type: 'revived' }, { type: 'died' }, { type: 'respawnReady' }]);
    expect(f.phase).toBe('Choosing');
  });

  it('a refused request returns to Choosing with the reason', () => {
    const f = new DeployFsm();
    run(f, [{ type: 'open' }, { type: 'request' }, { type: 'result', ok: false, reason: 'deploy.reject.full' }]);
    expect(f.state.phase).toBe('Choosing');
    expect(f.state.reason).toBe('deploy.reject.full');
  });

  it('every non-listed transition is rejected and leaves the state alone', () => {
    const phases: DeployPhase[] = ['Idle', 'Choosing', 'Requested', 'Counting', 'Spawned', 'Downed', 'Dead'];
    const evs: DeployEvent[] = [
      { type: 'open' }, { type: 'request' }, { type: 'result', ok: true, spawnTick: 1 }, { type: 'spawned' },
      { type: 'downed' }, { type: 'revived' }, { type: 'died' }, { type: 'respawnReady' },
    ];
    const legal = new Set([
      'Idle:open', 'Choosing:request', 'Requested:result', 'Counting:spawned', 'Counting:died', 'Spawned:downed',
      'Spawned:died', 'Downed:revived', 'Downed:died', 'Dead:respawnReady',
    ]);
    let checked = 0;
    for (const p of phases) {
      for (const e of evs) {
        const f = new DeployFsm();
        f.apply({ type: 'sync', phase: p });
        const r = f.apply(e);
        expect(r, `${p}:${e.type}`).toBe(legal.has(`${p}:${e.type}`));
        if (!r) expect(f.phase).toBe(p);
        checked++;
      }
    }
    expect(checked).toBe(56);
  });

  it('sync (self_state) and reset override', () => {
    const f = new DeployFsm();
    f.apply({ type: 'sync', phase: 'Spawned' });
    expect(f.phase).toBe('Spawned');
    f.apply({ type: 'reset' });
    expect(f.phase).toBe('Idle');
  });
});

describe('EventDedup', () => {
  it('dedup_c4: a repeated id passes once', () => {
    const d = new EventDedup();
    expect([d.firstTime(7), d.firstTime(7), d.firstTime(7)]).toEqual([true, false, false]);
    expect(d.duplicates).toBe(2);
  });

  it('forgets ids after the 256-entry window wraps', () => {
    const d = new EventDedup();
    d.firstTime(1000);
    for (let i = 0; i < 255; i++) d.firstTime(i);
    expect(d.firstTime(1000)).toBe(false); // still inside the window
    d.firstTime(300); // pushes id 1000 out
    expect(d.firstTime(1000)).toBe(true);
  });
});

describe('KillFeed', () => {
  it('keeps the 8 newest in order', () => {
    const k = new KillFeed();
    for (let i = 0; i < 11; i++) k.push(i, 100 + i, 1, false, 10);
    expect(k.length).toBe(8);
    expect(k.at(0)?.killer).toBe(3);
    expect(k.at(7)?.killer).toBe(10);
    expect(k.at(8)).toBeUndefined();
  });
});

describe('Roster', () => {
  const e = (id: number, seq: number, name?: string): RosterDelta => ({
    id,
    seq,
    entry: name === undefined ? undefined : { team: id % 2, squad: 1, role: 0, name },
  });

  it('property: 1000 shuffled deltas end equal to in-order application', () => {
    let s = 12345;
    const rnd = () => ((s = (s * 1664525 + 1013904223) >>> 0) / 2 ** 32);
    const deltas: RosterDelta[] = [];
    for (let i = 0; i < 1000; i++) {
      const id = Math.floor(rnd() * 40);
      deltas.push(e(id, i + 1, rnd() < 0.2 ? undefined : `p${id}-${i}`));
    }
    const ordered = new Roster();
    deltas.forEach((d) => ordered.apply(d));
    const shuffled = [...deltas];
    for (let i = shuffled.length - 1; i > 0; i--) {
      const j = Math.floor(rnd() * (i + 1));
      [shuffled[i], shuffled[j]] = [shuffled[j] as RosterDelta, shuffled[i] as RosterDelta];
    }
    const r = new Roster();
    shuffled.forEach((d) => r.apply(d));
    expect(r.list()).toEqual(ordered.list());
  });

  it('a stale delta cannot resurrect a removed player; replaceAll rebuilds', () => {
    const r = new Roster();
    r.apply(e(1, 2, 'a'));
    r.apply(e(1, 3));
    expect(r.apply(e(1, 2, 'a'))).toBe(false);
    expect(r.size).toBe(0);
    r.replaceAll([{ id: 5, team: 0, squad: 1, role: 0, name: 'z' }], 10);
    expect(r.list().map((x) => x.id)).toEqual([5]);
  });
});
