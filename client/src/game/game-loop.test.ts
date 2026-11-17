import { describe, expect, it } from 'vitest';
import { CAN_RUN, REQUIRE, HAVE_WASM, HAVE_TERRAIN, runScenario, standStill, walkCircle, type ScenarioResult } from './test-harness';

if (!CAN_RUN && REQUIRE) throw new Error(`FRONTLINE_REQUIRE_TERRAIN=1 but wasm pkg (${HAVE_WASM}) or terrain chunks (${HAVE_TERRAIN}) is missing`);

const link = (oneWayMs: number, jitterMs = 0, loss = 0) => ({ oneWayMs, jitterMs, loss });
const report = (name: string, r: ScenarioResult): void => {
  const u = [...r.updateMs].sort((a, b) => a - b);
  const p = (q: number): number => u[Math.min(u.length - 1, Math.floor(u.length * q))] as number;
  console.log(`[${name}] ${JSON.stringify({ corrections: r.corrections, snaps: r.snaps, initial: r.initialSyncs, tolKept: r.toleranceKept, maxErrM: +r.maxErrM.toFixed(3), maxVisibleM: +r.maxVisibleOffsetM.toFixed(3), maxJumpM: +r.maxFrameJumpM.toFixed(3), serverRepeated: r.serverRepeated, bundlesLost: r.bundlesLost, snapsLost: r.snapshotsLost, extras: r.tickExtras, skips: r.tickSkips, replayed: r.replayedCommands, updP50ms: +p(0.5).toFixed(3), updP99ms: +p(0.99).toFixed(3) })}`);
};

describe.skipIf(!CAN_RUN)('replay_recorded: wasm Predictor as the server (100 ms ack delay, 2% snapshot loss)', () => {
  // acceptance 2: input always arrives; the snapshot path is delayed 100 ms and loses 2%.
  const ACK_DELAY = { up: link(0), down: link(100, 0, 0.02) };

  it('stationary 60 s: <= 6 corrections (1 per 10 s), no snaps after the initial placement', async () => {
    const r = await runScenario({ seconds: 60, ...ACK_DELAY, script: standStill, seed: 7 });
    report('static60', r);
    expect(r.initialSyncs).toBe(1);
    expect(r.corrections).toBeLessThanOrEqual(6);
    expect(r.snaps).toBe(0);
  }, 120_000);

  it('walking 60 s: no snaps and (identical sim) no corrections', async () => {
    const r = await runScenario({ seconds: 60, ...ACK_DELAY, script: walkCircle, seed: 9 });
    report('walk60', r);
    expect(r.initialSyncs).toBe(1);
    expect(r.snaps).toBe(0);
    expect(r.corrections).toBeLessThanOrEqual(6);
    expect(r.finalPosErrM).toBeLessThan(0.5);
  }, 120_000);
});
