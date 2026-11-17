import { describe, expect, it } from 'vitest';
import { EventBus, type GameEvents } from './event-bus';
import { GameLoop, type ClockPort } from './game-loop';
import { CAN_RUN, FakeSnapshotDecoder, loadChunks, loadWasm, walkCircle, ScriptInput } from './test-harness';

/** Cost of one prediction step and of one rollback + replay (informational; generous limits because CI machines are noisy). */
describe.skipIf(!CAN_RUN)('prediction / reconciliation cost (real sim-wasm, Node)', () => {
  it('live step (input -> cmd -> predict -> ring -> bundle) and rollback with 8 / 16 / 32 replayed commands', async () => {
    const wasm = await loadWasm();
    let tick = 1000;
    const clock: ClockPort = { inputTargetTick: () => tick, estimatedServerTick: () => tick - 4, now: () => tick, applySlack: () => undefined, rtt: () => 100 };
    const sent: number[] = [];
    const g = new GameLoop({
      wasm,
      decoder: new FakeSnapshotDecoder(),
      bus: new EventBus<GameEvents>(),
      net: { clock, sendInputBundle: (b) => sent.push(b.length) },
      input: new ScriptInput(walkCircle),
      matchSeed: 1n,
      mapId: 0,
      playerId: 3,
      now: () => tick * (1000 / 30),
      log: () => undefined,
    });
    g.loadCollision(loadChunks());
    g.reconciler.spawned = true;
    const stepOnce = (): void => {
      tick++;
      g.update(1, 0.5, 33.3);
    };
    for (let i = 0; i < 300; i++) stepOnce(); // warm-up / JIT
    const N = 3000;
    const live = new Float64Array(N);
    for (let i = 0; i < N; i++) {
      const a = performance.now();
      stepOnce();
      live[i] = performance.now() - a;
    }
    const pct = (v: Float64Array, q: number): number => Array.from(v).sort((x, y) => x - y)[Math.floor(v.length * q)] as number;
    const out: string[] = [`live step+bundle: P50 ${pct(live, 0.5).toFixed(4)} ms, P99 ${pct(live, 0.99).toFixed(4)} ms (n=${N})`];
    for (const pending of [8, 16, 32]) {
      const T = 400;
      const t = new Float64Array(T);
      for (let k = 0; k < T; k++) {
        while (g.ring.length < pending + 1) stepOnce();
        const newest = g.ring.newest()!.seq;
        const ack = (newest - pending + 0x10000) & 0xffff;
        const ref = g.ring.stateAt(ack)!;
        const a = performance.now();
        const r = g.reconciler.onSelf(ack, true, ref.x + (k % 2 ? -0.2 : 0.2), ref.y, ref.z);
        t[k] = performance.now() - a;
        if (r.kind !== 'corrected') throw new Error(`k=${k} pending=${pending} kind=${r.kind} err=${r.err} err0=${r.err0}`);
        stepOnce();
      }
      out.push(`rollback+replay ${pending} cmds: P50 ${pct(t, 0.5).toFixed(4)} ms, P99 ${pct(t, 0.99).toFixed(4)} ms`);
      expect(pct(t, 0.5)).toBeLessThan(2);
    }
    console.log(`[cost] ${out.join(' | ')}`);
    expect(pct(live, 0.99)).toBeLessThan(2);
    expect(sent.length).toBeGreaterThan(N);
    g.dispose();
  }, 120_000);
});
