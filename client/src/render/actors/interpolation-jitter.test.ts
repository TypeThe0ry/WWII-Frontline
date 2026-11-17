import { describe, expect, it } from 'vitest';
import { CAN_RUN, runScenario, standStill } from '../../game/test-harness';
import { ActorManager } from './actor-manager';
import { makeScene, realData } from './test-support';

// Acceptance 4: 10 remote entities (constant-velocity bots standing in for M0-026's bots), snapshots at 20 Hz with
// 10 % jitter and 2 % loss, 60 fps rendering for 60 s: the rendered positions (the actor nodes) have a second difference
// RMS <= 0.02 m and never jump by more than 1 m in a frame.
describe.skipIf(!CAN_RUN)('remote actor motion under network jitter', () => {
  it('10 remote actors, 20 Hz snapshots with 10 % jitter and 2 % loss, 60 fps, 60 s: second-difference RMS <= 0.02 m, no teleports', async () => {
    const { data } = await realData();
    const ts = makeScene();
    let mgr: ActorManager | undefined;
    const BOTS = 10;
    // per actor: [x(n-1), z(n-1), x(n-2), z(n-2), frames seen]
    const hist = new Map<number, number[]>();
    const sd: number[] = [];
    let maxStep = 0;
    let samples = 0;
    const r = await runScenario({
      seconds: 60,
      up: { oneWayMs: 25, jitterMs: 0, loss: 0 },
      down: { oneWayMs: 25, jitterMs: 5, loss: 0.02 }, // 5 ms = 10 % of the 50 ms snapshot interval
      script: standStill,
      seed: 5,
      bots: BOTS,
      setup: (game, bus) => {
        mgr = new ActorManager({ scene: ts.scene, bus, state: game.state, data, now: () => 0 });
      },
      onFrame: (_g, _f, frameMs, tMs) => {
        mgr!.update(1, frameMs);
        if (tMs < 3000) return; // skip the start-up (clock sync, buffer fill)
        for (const s of mgr!.allActors()) {
          const p = s.actor.root.position;
          const h = hist.get(s.actor.id) ?? [p.x, p.z, p.x, p.z, 0];
          if ((h[4] as number) >= 2) {
            const ddx = p.x - 2 * (h[0] as number) + (h[2] as number);
            const ddz = p.z - 2 * (h[1] as number) + (h[3] as number);
            sd.push(ddx * ddx + ddz * ddz);
            samples++;
            maxStep = Math.max(maxStep, Math.hypot(p.x - (h[0] as number), p.z - (h[1] as number)));
          }
          hist.set(s.actor.id, [p.x, p.z, h[0] as number, h[1] as number, (h[4] as number) + 1]);
        }
      },
    });
    const rms = Math.sqrt(sd.reduce((a, b) => a + b, 0) / Math.max(1, sd.length));
    console.log(`[actor interpolation 60 s] actors ${mgr!.actorCount} samples ${samples} secondDiffRMS ${rms.toFixed(5)} m maxFrameStep ${maxStep.toFixed(3)} m snapshots sent ${r.snapshotsSent} lost ${r.snapshotsLost}`);
    expect(mgr!.actorCount).toBe(BOTS);
    expect(samples).toBeGreaterThan(BOTS * 60 * 50);
    expect(rms).toBeLessThanOrEqual(0.02);
    expect(maxStep).toBeLessThan(1); // 0 teleports: no single-frame displacement above 1 m (bots move 2-4 m/s: 0.07 m per frame)
    mgr!.dispose();
    ts.dispose();
  }, 120_000);
});
