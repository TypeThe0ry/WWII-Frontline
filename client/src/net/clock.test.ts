import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ClockSync, ewma, MAX_INPUT_LEAD_TICKS, MAX_SLACK_RATE, TICK_MS } from './clock';

describe('RTT EWMA (alpha 0.1)', () => {
  it('[100,100,200,100] matches the hand calculation to 0.01 ms', () => {
    // seed 100; 100 -> 100; 200 -> 100 + 0.1*(200-100) = 110; 100 -> 110 + 0.1*(100-110) = 109
    const expected = [100, 100, 110, 109];
    let v: number | undefined;
    [100, 100, 200, 100].forEach((s, i) => {
      v = ewma(v, s);
      expect(v).toBeCloseTo(expected[i] as number, 2);
    });
  });

  it('ClockSync.rtt() follows the same sequence through Ping/Pong', () => {
    let t = 1000;
    const sent: number[] = [];
    const c = new ClockSync({ now: () => t, sendPing: (ct) => sent.push(ct) });
    const got: number[] = [];
    for (const rtt of [100, 100, 200, 100]) {
      c.firePing();
      const ct = sent[sent.length - 1] as number;
      t += rtt;
      c.onPong(ct, 50, t);
      got.push(c.rtt());
      t += 10;
    }
    expect(got.map((v) => Math.round(v * 100) / 100)).toEqual([100, 100, 110, 109]);
  });
});

describe('ClockSync misc', () => {
  it('ignores unknown / duplicate Pongs and computes jitter as the RTT standard deviation', () => {
    let t = 0;
    const sent: number[] = [];
    const c = new ClockSync({ now: () => t, sendPing: (ct) => sent.push(ct) });
    expect(c.onPong(999, 1, 5)).toBeUndefined();
    for (const rtt of [80, 100, 80, 100]) {
      c.firePing();
      t += rtt;
      expect(c.onPong(sent[sent.length - 1] as number, 5, t)).toBe(rtt);
      expect(c.onPong(sent[sent.length - 1] as number, 5, t)).toBeUndefined(); // duplicate
      t += 5;
    }
    expect(c.jitter()).toBeCloseTo(10, 6);
  });

  it('client_time stays strictly increasing within one millisecond', () => {
    const sent: number[] = [];
    const c = new ClockSync({ now: () => 5000.2, sendPing: (ct) => sent.push(ct) });
    c.firePing();
    c.firePing();
    c.firePing();
    expect(sent).toEqual([5000, 5001, 5002]);
  });

  it('input lead = ceil(rtt/2 / 33.33 ms) + 1', () => {
    const c = new ClockSync({ now: () => 0 });
    c.seed(0, 80, 0);
    expect(c.inputLeadTicks()).toBe(Math.ceil(40 / TICK_MS) + 1); // 3
    c.seed(0, 0, 0);
    const d = new ClockSync({ now: () => 0 });
    d.seed(0, 200, 0);
    expect(d.inputLeadTicks()).toBe(Math.ceil(100 / TICK_MS) + 1); // 5
  });
});

describe('input lead cap (M0-030)', () => {
  it('never exceeds what the server accepts, however inflated the RTT of a 1-5 fps client gets', () => {
    for (const rtt of [600, 1000, 3000]) {
      const c = new ClockSync({ now: () => 0 });
      c.seed(0, rtt, 0);
      expect(c.inputLeadTicks()).toBe(MAX_INPUT_LEAD_TICKS);
    }
    const ok = new ClockSync({ now: () => 0 });
    ok.seed(0, 300, 0);
    expect(ok.inputLeadTicks()).toBe(Math.ceil(150 / TICK_MS) + 1); // 6: below the cap, unchanged
  });
});

describe('applySlack', () => {
  it('moves the lead correction by at most 10% of a tick per second and at most +-1 tick', () => {
    let t = 0;
    const c = new ClockSync({ now: () => t });
    c.seed(0, 80, 0);
    const base = c.inputLeadTicks();
    c.applySlack(0, t); // too late (slack 0 < target 1): wants +1, first call has dt = 0
    expect(c.slackCorrection).toBe(0);
    t += 1000;
    c.applySlack(0, t);
    expect(c.slackCorrection).toBeCloseTo(MAX_SLACK_RATE, 9);
    for (let i = 0; i < 40; i++) {
      t += 1000;
      c.applySlack(0, t);
    }
    expect(c.slackCorrection).toBe(1); // clamped at +1 tick
    expect(c.inputLeadTicks()).toBe(base + 1);
    // far too early (slack 15): wants -1; slews back at 0.1 tick/s
    t += 1000;
    c.applySlack(15, t);
    expect(c.slackCorrection).toBeCloseTo(1 - MAX_SLACK_RATE, 9);
    for (let i = 0; i < 40; i++) {
      t += 1000;
      c.applySlack(15, t);
    }
    expect(c.slackCorrection).toBe(-1);
    expect(c.inputLeadTicks()).toBe(base - 1);
  });

  it('a burst of feedback inside one second cannot exceed the rate limit', () => {
    let t = 0;
    const c = new ClockSync({ now: () => t });
    c.seed(0, 80, 0);
    c.applySlack(0, t);
    for (let i = 0; i < 30; i++) {
      t += 1000 / 30;
      c.applySlack(0, t);
    }
    expect(c.slackCorrection).toBeLessThanOrEqual(MAX_SLACK_RATE + 1e-9);
  });

  it('slack at the target leaves the correction at zero', () => {
    let t = 0;
    const c = new ClockSync({ now: () => t });
    for (let i = 0; i < 10; i++) {
      t += 1000;
      c.applySlack(1, t);
    }
    expect(c.slackCorrection).toBe(0);
  });
});

describe('clock_sync: 80 ms RTT, +-10 ms jitter, 60 s (fake timers)', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  /** Deterministic PRNG so the run is reproducible. */
  function rng(seed: number): () => number {
    let s = seed >>> 0;
    return () => {
      s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
      return s / 4294967296;
    };
  }

  function run(seed: number, serverAheadTicks: number): { worst: number; within: number; total: number } {
    const rand = rng(seed);
    const jit = (): number => (rand() * 2 - 1) * 10; // uniform +-10 ms on every leg
    const serverTickAt = (ms: number): number => serverAheadTicks + ms / TICK_MS; // continuous truth
    const start = 5000; // fake clock time of connect
    vi.setSystemTime(start);
    const now = (): number => Date.now();
    const c = new ClockSync({
      now,
      sendPing: (ct) => {
        const sentAt = now();
        const fwd = 40 + jit();
        const back = 40 + jit();
        const serverAt = sentAt + fwd;
        // The server answers with its integer tick at the moment it handles the Ping.
        const tick = Math.floor(serverTickAt(serverAt));
        setTimeout(() => c.onPong(ct, tick), fwd + back);
      },
    });
    // Welcome: server tick as of 40 ms ago (+- jitter), received "now".
    const welcomeRtt = 80 + jit() + jit();
    c.seed(Math.floor(serverTickAt(now() - welcomeRtt / 2)), welcomeRtt);
    c.start();
    let worst = 0;
    let within = 0;
    let total = 0;
    for (let t = 0; t < 60_000; t += 100) {
      vi.advanceTimersByTime(100);
      if (t + 100 >= 5000) {
        const truth = Math.floor(serverTickAt(now()));
        const err = Math.abs(c.estimatedServerTick() - truth);
        worst = Math.max(worst, err);
        total++;
        if (err <= 1) within++;
      }
    }
    c.stop();
    return { worst, within, total };
  }

  it('|estimate - truth| <= 1 tick for 100 % of the samples from 5 s after connecting', () => {
    for (const seed of [1, 2, 3, 42, 2026]) {
      const r = run(seed, 90_000 + seed);
      expect(r.total).toBe(551);
      expect(r.within).toBe(r.total);
      console.info(`clock_sync seed ${seed}: ${r.within}/${r.total} samples within 1 tick, worst error ${r.worst} tick`);
    }
  });

  it('starts pinging with 5 quick pings inside 500 ms, then 1 Hz', () => {
    const times: number[] = [];
    vi.setSystemTime(0);
    const c = new ClockSync({ now: () => Date.now(), sendPing: () => times.push(Date.now()) });
    c.start();
    vi.advanceTimersByTime(3500);
    c.stop();
    expect(times).toEqual([0, 100, 200, 300, 400, 1400, 2400, 3400]);
  });
});
