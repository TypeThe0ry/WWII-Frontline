/**
 * Server clock estimate (ARCHITECTURE §6.2).
 *
 * Ping/Pong every 1 s (5 quick pings, 100 ms apart, right after connecting). Each Pong carries the
 * server tick at the moment the server answered; with the round trip `rtt` the server tick "now" is
 *
 *     tick_pong + 0.5 + (now - t_pong_recv + rtt / 2) / TICK_MS
 *
 * (+0.5: the Pong tick is the integer floor of the server's continuous tick, so its expected value is
 * half a tick low). The estimate uses, of the last PONG_WINDOW Pongs, the one with the smallest RTT
 * (least queueing, so its half-RTT assumption is the most accurate), with that Pong's own RTT.
 * `rtt()` is the EWMA (alpha 0.1) over all samples and drives the input lead.
 *
 * Input lead: `ceil(rtt/2 / TICK_MS) + 1` ticks plus a slack correction. The server reports how many
 * ticks before execution the latest input arrived (`input_slack`, 4 bit); `applySlack` nudges the
 * lead towards `TARGET_SLACK` by at most +-1 tick overall and at most 10 % of a tick per second.
 */
export const TICK_HZ = 30;
export const TICK_MS = 1000 / TICK_HZ;
export const RTT_ALPHA = 0.1;
export const PING_INTERVAL_MS = 1000;
export const QUICK_PINGS = 5;
export const QUICK_PING_INTERVAL_MS = 100;
export const TARGET_SLACK = 1;
export const MAX_SLACK_BIAS_TICKS = 1;
/** Slack correction may change by at most this many ticks per second (10 %/s). */
export const MAX_SLACK_RATE = 0.1;
const PONG_WINDOW = 8;
const JITTER_WINDOW = 32;

/** One EWMA step. The first sample seeds the average. */
export function ewma(prev: number | undefined, sample: number, alpha = RTT_ALPHA): number {
  return prev === undefined ? sample : prev + alpha * (sample - prev);
}

export interface ClockOptions {
  /** Monotonic ms clock (default `performance.now`). */
  now?: () => number;
  /** Sends a Ping carrying `clientTime` (a u32) on C0. */
  sendPing?: (clientTime: number) => void;
  tickMs?: number;
}

interface PongSample {
  tick: number;
  recvAt: number;
  rtt: number;
}

export class ClockSync {
  readonly tickMs: number;
  private readonly clock: () => number;
  private readonly sendPing: (clientTime: number) => void;

  private rttEwma: number | undefined;
  private readonly rtts: number[] = [];
  private readonly pongs: PongSample[] = [];
  private seedSample: PongSample | undefined;
  private pending = new Map<number, number>();
  private lastClientTime = 0;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private pingsSent = 0;

  private slackBias = 0;
  private lastSlackAt: number | undefined;

  constructor(opts: ClockOptions = {}) {
    this.tickMs = opts.tickMs ?? TICK_MS;
    this.clock = opts.now ?? (() => performance.now());
    this.sendPing = opts.sendPing ?? (() => undefined);
  }

  /** Initial estimate from `Welcome.server_tick`, received at the local time `recvAt` after a handshake round trip of `rttMs`. */
  seed(serverTick: number, rttMs: number, recvAt = this.clock()): void {
    this.rttEwma = ewma(this.rttEwma, rttMs);
    this.pushRtt(rttMs);
    this.seedSample = { tick: serverTick, recvAt, rtt: rttMs };
  }

  /** Starts the ping schedule (5 quick pings in the first 400 ms, then 1 Hz). */
  start(): void {
    this.stop();
    this.pingsSent = 0;
    this.tickTimer();
  }

  stop(): void {
    if (this.timer !== undefined) clearTimeout(this.timer);
    this.timer = undefined;
  }

  private tickTimer(): void {
    this.firePing();
    this.pingsSent++;
    const next = this.pingsSent < QUICK_PINGS ? QUICK_PING_INTERVAL_MS : PING_INTERVAL_MS;
    this.timer = setTimeout(() => this.tickTimer(), next);
  }

  /** Sends one Ping now. */
  firePing(): void {
    const t = this.clock();
    // u32 client time in ms; strictly increasing so a Pong maps back to its exact send time.
    let ct = Math.floor(t) >>> 0;
    if (ct <= this.lastClientTime && this.lastClientTime - ct < 0x8000_0000) ct = (this.lastClientTime + 1) >>> 0;
    this.lastClientTime = ct;
    this.pending.set(ct, t);
    if (this.pending.size > 64) this.pending.delete(this.pending.keys().next().value as number);
    this.sendPing(ct);
  }

  /** Feeds a Pong. Returns the RTT sample in ms, or undefined for an unknown/duplicate one. */
  onPong(clientTime: number, serverTick: number, recvAt = this.clock()): number | undefined {
    const sentAt = this.pending.get(clientTime);
    if (sentAt === undefined) return undefined;
    this.pending.delete(clientTime);
    const rtt = Math.max(0, recvAt - sentAt);
    this.rttEwma = ewma(this.rttEwma, rtt);
    this.pushRtt(rtt);
    this.pongs.push({ tick: serverTick, recvAt, rtt });
    if (this.pongs.length > PONG_WINDOW) this.pongs.shift();
    return rtt;
  }

  private pushRtt(r: number): void {
    this.rtts.push(r);
    if (this.rtts.length > JITTER_WINDOW) this.rtts.shift();
  }

  private best(): PongSample | undefined {
    let b: PongSample | undefined;
    for (const p of this.pongs) if (!b || p.rtt < b.rtt) b = p;
    return b ?? this.seedSample;
  }

  get synced(): boolean {
    return this.best() !== undefined;
  }

  /** Smoothed round-trip time in ms (EWMA alpha 0.1); 0 before the first sample. */
  rtt(): number {
    return this.rttEwma ?? 0;
  }

  /** Standard deviation of the recent RTT samples in ms. */
  jitter(): number {
    const n = this.rtts.length;
    if (n < 2) return 0;
    let mean = 0;
    for (const r of this.rtts) mean += r;
    mean /= n;
    let v = 0;
    for (const r of this.rtts) v += (r - mean) * (r - mean);
    return Math.sqrt(v / n);
  }

  /** Estimated current server tick as a continuous value (`NaN` before the first sample). */
  now(at = this.clock()): number {
    const b = this.best();
    if (!b) return NaN;
    return b.tick + 0.5 + (at - b.recvAt + b.rtt / 2) / this.tickMs;
  }

  /** `estimated_server_tick`, rounded down to the tick that is currently running. */
  estimatedServerTick(at = this.clock()): number {
    return Math.floor(this.now(at));
  }

  /** Ticks the client must run ahead of the server for inputs to arrive in time. */
  inputLeadTicks(): number {
    return Math.ceil(this.rtt() / 2 / this.tickMs) + 1 + Math.round(this.slackBias);
  }

  /** The tick to stamp on inputs generated now. */
  inputTargetTick(at = this.clock()): number {
    return this.estimatedServerTick(at) + this.inputLeadTicks();
  }

  get slackCorrection(): number {
    return this.slackBias;
  }

  /**
   * Server feedback from the snapshot header: how many ticks early the last input arrived. Too late
   * (slack below target) raises the lead, too early lowers it; +-1 tick in total, 0.1 tick per second.
   */
  applySlack(slack: number, at = this.clock()): void {
    const dt = this.lastSlackAt === undefined ? 0 : Math.max(0, at - this.lastSlackAt) / 1000;
    this.lastSlackAt = at;
    const want = Math.max(-MAX_SLACK_BIAS_TICKS, Math.min(MAX_SLACK_BIAS_TICKS, TARGET_SLACK - slack));
    const maxStep = MAX_SLACK_RATE * dt;
    const diff = want - this.slackBias;
    this.slackBias += Math.max(-maxStep, Math.min(maxStep, diff));
  }
}
