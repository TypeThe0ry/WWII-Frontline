import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { CAN_RUN, HAVE_TERRAIN, HAVE_WASM, REQUIRE, replayFixture, runScenario, standStill, walkCircle, type Fixture, type RecordedSnap, type Scenario, type ScenarioResult } from './test-harness';

if (!CAN_RUN && REQUIRE) throw new Error(`FRONTLINE_REQUIRE_TERRAIN=1 but wasm pkg (${HAVE_WASM}) or terrain chunks (${HAVE_TERRAIN}) is missing`);

const link = (oneWayMs: number, jitterMs = 0, loss = 0) => ({ oneWayMs, jitterMs, loss });
const lines: string[] = [];
const report = (name: string, r: ScenarioResult): void => {
  const u = [...r.updateMs].sort((a, b) => a - b);
  const p = (q: number): number => u[Math.min(u.length - 1, Math.floor(u.length * q))] as number;
  const line = `[${name}] ${JSON.stringify({ corrections: r.corrections, steady: r.correctionTimesS.filter((t) => t > 5).length, snaps: r.snaps, tolKept: r.toleranceKept, maxErrM: +r.maxErrM.toFixed(3), maxVisibleM: +r.maxVisibleOffsetM.toFixed(3), maxJumpM: +r.maxFrameJumpM.toFixed(3), serverRepeated: r.serverRepeated, bundlesLost: r.bundlesLost, snapsLost: r.snapshotsLost, extras: r.tickExtras, skips: r.tickSkips, replayed: r.replayedCommands, pathM: Math.round(r.pathM), updP50ms: +p(0.5).toFixed(3), updP99ms: +p(0.99).toFixed(3) })}`;
  lines.push(line);
  console.log(line);
};
const steady = (r: ScenarioResult): number => r.correctionTimesS.filter((t) => t > 5).length;
const base = (over: Partial<Scenario>): Scenario => ({ seconds: 30, up: link(0), down: link(0), script: standStill, ...over });

describe.skipIf(!CAN_RUN)('replay_recorded: wasm Predictor as the server (100 ms ack delay, 2% snapshot loss)', () => {
  // acceptance 2: input always arrives; the snapshot path is delayed 100 ms and loses 2%.
  const ACK_DELAY = { up: link(0), down: link(100, 0, 0.02) };

  it('stationary 60 s: <= 6 corrections (1 per 10 s), no snaps after the initial placement', async () => {
    const r = await runScenario(base({ seconds: 60, ...ACK_DELAY, script: standStill, seed: 7 }));
    report('static60 ack100ms loss2%', r);
    expect(r.initialSyncs).toBe(1);
    expect(r.corrections).toBeLessThanOrEqual(6);
    expect(r.snaps).toBe(0);
    expect(steady(r)).toBe(0); // identical sim: nothing to correct after the start-up sync
  }, 120_000);

  it('walking 60 s: no snaps, no steady-state corrections, the player really moved', async () => {
    const r = await runScenario(base({ seconds: 60, ...ACK_DELAY, script: walkCircle, seed: 9 }));
    report('walk60 ack100ms loss2%', r);
    expect(r.initialSyncs).toBe(1);
    expect(r.snaps).toBe(0);
    expect(r.corrections).toBeLessThanOrEqual(6);
    expect(steady(r)).toBe(0);
    expect(r.pathM).toBeGreaterThan(200);
    expect(r.maxFrameJumpM).toBeLessThan(0.3);
  }, 120_000);
});

describe.skipIf(!CAN_RUN)('network matrix (latency / jitter / loss, input also crosses the lossy link)', () => {
  const cases: { name: string; sc: Partial<Scenario>; maxSteady: number }[] = [
    { name: 'rtt40 clean', sc: { up: link(20), down: link(20) }, maxSteady: 0 },
    { name: 'rtt100 loss2% (both ways)', sc: { up: link(50, 0, 0.02), down: link(50, 0, 0.02) }, maxSteady: 3 },
    { name: 'rtt100 jitter20ms loss2%', sc: { up: link(50, 20, 0.02), down: link(50, 20, 0.02) }, maxSteady: 12 },
    { name: 'rtt200 jitter40ms loss5%', sc: { up: link(100, 40, 0.05), down: link(100, 40, 0.05) }, maxSteady: 40 },
    { name: 'rtt100 loss10%', sc: { up: link(50, 10, 0.1), down: link(50, 10, 0.1) }, maxSteady: 40 },
  ];
  for (const c of cases) {
    it(`${c.name}: walking 60 s`, async () => {
      const r = await runScenario(base({ seconds: 60, script: walkCircle, seed: 11, ...c.sc }));
      report(`matrix walk60 ${c.name}`, r);
      expect(r.snaps).toBe(0);
      expect(steady(r)).toBeLessThanOrEqual(c.maxSteady);
      expect(r.maxVisibleOffsetM).toBeLessThan(1);
    }, 120_000);
  }

  it('rtt100 jitter20ms loss2%: stationary 120 s keeps corrections <= 12', async () => {
    const r = await runScenario(base({ seconds: 120, up: link(50, 20, 0.02), down: link(50, 20, 0.02), script: standStill, seed: 5 }));
    report('static120 rtt100 jitter20ms loss2%', r);
    expect(r.corrections).toBeLessThanOrEqual(12);
  }, 120_000);
});

describe.skipIf(!CAN_RUN)('injected deviation: correction and smoothing', () => {
  it('a 0.3 m server-side displacement every 10 s is corrected without a snap and smoothed out', async () => {
    const r = await runScenario(base({ seconds: 60, up: link(50), down: link(50), script: standStill, seed: 3, kick: { everyS: 10, metres: 0.3 } }));
    report('kick0.3m/10s', r);
    expect(r.snaps).toBe(0);
    expect(steady(r)).toBeGreaterThanOrEqual(5);
    expect(steady(r)).toBeLessThanOrEqual(7);
    expect(r.maxErrM).toBeGreaterThan(0.25);
    expect(r.maxErrM).toBeLessThan(0.35);
    expect(r.maxVisibleOffsetM).toBeLessThan(0.35); // the offset is the correction, never more
  }, 120_000);

  it('a single 0.03 m displacement is inside the tolerance: no rollback (repeated ones add up past 0.05 m and are then corrected)', async () => {
    const r = await runScenario(base({ seconds: 18, up: link(50), down: link(50), script: standStill, seed: 3, kick: { everyS: 10, metres: 0.03 } }));
    report('kick0.03m/10s', r);
    expect(steady(r)).toBe(0);
    expect(r.toleranceKept).toBeGreaterThan(0);
  }, 120_000);

  it('a 1.5 m displacement snaps (no smoothing)', async () => {
    const r = await runScenario(base({ seconds: 40, up: link(50), down: link(50), script: standStill, seed: 3, kick: { everyS: 10, metres: 1.5 } }));
    report('kick1.5m/10s', r);
    expect(r.snaps).toBeGreaterThanOrEqual(3);
    expect(r.maxVisibleOffsetM).toBeLessThan(0.05);
  }, 120_000);

  it('walking while the server pushes 0.3 m: corrections stay bounded and the visible error below 0.35 m', async () => {
    const r = await runScenario(base({ seconds: 60, up: link(50, 10, 0.02), down: link(50, 10, 0.02), script: walkCircle, seed: 4, kick: { everyS: 10, metres: 0.3 } }));
    report('walk kick0.3m/10s rtt100 loss2%', r);
    expect(r.snaps).toBe(0);
    expect(r.maxVisibleOffsetM).toBeLessThan(0.4);
  }, 120_000);
});

describe.skipIf(!CAN_RUN)('input tick planning (lead follows the clock estimate)', () => {
  it('a constant clock-estimate offset is absorbed at start; a drifting estimate triggers few adjustments', async () => {
    const r = await runScenario(base({ seconds: 60, up: link(50, 5), down: link(50, 5), script: walkCircle, seed: 2, clockWobbleTicks: 1.5 }));
    report('wobble1.5 ticks', r);
    expect(r.tickExtras + r.tickSkips).toBeLessThan(40);
    expect(r.snaps).toBe(0);
  }, 120_000);
});

describe.skipIf(!CAN_RUN)('bundle size (redundancy: newest 3 unacknowledged commands)', () => {
  it('measured sizes for stationary / walking-and-turning input', async () => {
    const a = await runScenario(base({ seconds: 20, up: link(50), down: link(50), script: standStill }));
    const b = await runScenario(base({ seconds: 20, up: link(50), down: link(50), script: walkCircle }));
    const avg = (r: ScenarioResult, n: number): number => ((r.bytesByN[n] as number) / Math.max(1, r.countByN[n] as number));
    console.log(`[bundle bytes] stationary avg ${a.bundleBytesAvg.toFixed(1)} max ${a.bundleBytesMax}; walking avg ${b.bundleBytesAvg.toFixed(1)} max ${b.bundleBytesMax}; by n (walking) 1:${avg(b, 1).toFixed(1)} 2:${avg(b, 2).toFixed(1)} 3:${avg(b, 3).toFixed(1)}; kbps(30 pkt/s) stationary ${((a.bundleBytesAvg * 30 * 8) / 1000).toFixed(1)} walking ${((b.bundleBytesAvg * 30 * 8) / 1000).toFixed(1)}`);
    expect(b.bundleBytesMax).toBeLessThanOrEqual(30); // M0-005 budget for three typical commands
    expect(b.countByN[3]).toBeGreaterThan(b.bundles * 0.9);
  }, 60_000);
});

describe.skipIf(!CAN_RUN)('main-thread cost (acceptance 7: game update + prediction 0-2 steps + decode + interpolation, 60 entities)', () => {
  it('P99 per frame <= 2.1 ms with 60 remote entities, walking, 2% loss', async () => {
    const r = await runScenario(base({ seconds: 30, up: link(50, 10, 0.02), down: link(50, 10, 0.02), script: walkCircle, seed: 8, bots: 60 }));
    report('perf60 entities', r);
    const u = r.updateMs.slice(120).sort((a, b) => a - b);
    const q = (x: number): number => u[Math.floor(u.length * x)] as number;
    console.log(`[perf] frames ${u.length}: P50 ${q(0.5).toFixed(3)} ms, P99 ${q(0.99).toFixed(3)} ms, max ${(u[u.length - 1] as number).toFixed(3)} ms (budget 2.1 ms; includes the fake-decoder allocations and the Node timer)`);
    expect(r.game.state.entities.count).toBe(60);
    expect(q(0.99)).toBeLessThanOrEqual(2.1);
  }, 120_000);
});

describe.skipIf(!CAN_RUN)('regression fixture (recorded snapshot sequence, replayed without a server)', () => {
  const FIXTURE = fileURLToPath(new URL('../../tests/fixtures/prediction-walk-kick.json', import.meta.url));
  const scenario = (record: RecordedSnap[]): Scenario => base({ seconds: 20, up: link(50, 10, 0.02), down: link(50, 10, 0.02), script: walkCircle, seed: 21, kick: { everyS: 8, metres: 0.3 }, record });

  it('replaying the stored run reproduces its reconciliation exactly', async () => {
    if (process.env.FRONTLINE_UPDATE_FIXTURES === '1') {
      const record: RecordedSnap[] = [];
      const r = await runScenario(scenario(record));
      const fx: Fixture = {
        description: 'M0-022 regression: 20 s of walking in a circle, RTT 100 ms (+-10 ms jitter), 2 % loss, the server pushes the player 0.3 m every 8 s. Snapshots as delivered to the client (frame index f at 60 fps).',
        frameMs: 1000 / 60,
        frames: Math.floor(20000 / (1000 / 60)),
        script: 'walkCircle',
        rttMs: 110,
        expected: { corrections: 0, snaps: 0, initialSyncs: 0, toleranceKept: 0, replayedCommands: 0, bundles: 0 },
        snaps: record,
      };
      // the stored expectation comes from replaying the fixture itself (the clock model differs slightly from the live run)
      const { game } = await replayFixture(fx);
      const s = game.reconciler.stats;
      fx.expected = { corrections: s.corrections, snaps: s.snaps, initialSyncs: s.initialSyncs, toleranceKept: s.toleranceKept, replayedCommands: s.replayedCommands, bundles: game.stats.bundles };
      writeFileSync(FIXTURE, JSON.stringify(fx));
      console.log(`[fixture] written ${record.length} snapshots, live run corrections ${r.corrections}, replay ${JSON.stringify(fx.expected)}`);
    }
    const fx = JSON.parse(readFileSync(FIXTURE, 'utf8')) as Fixture;
    const { game } = await replayFixture(fx);
    const s = game.reconciler.stats;
    expect({ corrections: s.corrections, snaps: s.snaps, initialSyncs: s.initialSyncs, toleranceKept: s.toleranceKept, replayedCommands: s.replayedCommands, bundles: game.stats.bundles }).toEqual(fx.expected);
    expect(s.initialSyncs).toBe(1);
    expect(s.snaps).toBe(0);
    expect(s.corrections).toBeGreaterThanOrEqual(2); // the two 0.3 m pushes
  });
});
