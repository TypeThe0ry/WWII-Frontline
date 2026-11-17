// In-process fake server + network for the game tests (vitest only; not imported by the app).
//
// The authoritative end is the same sim-wasm `Predictor` the client predicts with, fed with the commands
// that arrive over a simulated link (latency, jitter, loss, reordering). Snapshots are not byte-encoded
// (the wasm package has no encoder): the fake server hands the client a small id that a
// `FakeSnapshotDecoder` resolves to the snapshot fields, so everything from `GameLoop.onSnapshot` on is
// the production code path (the real wasm decoder is covered by M0-014 and the M0-025/027 end-to-end runs).
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { EventBus, type GameEvents } from './event-bus';
import { FixedStepper } from './fixed-step';
import { GameLoop, type ClockPort, type NetPort } from './game-loop';
import { predHash } from './pred-hash';
import type { InputCmdFields, InputSource } from '../input/sampler';
import { LAYOUT_SELF_LEN, type SimWasmModule, type WasmPredictor, type WasmSnapshotDecoder } from '../net/wasm';
import { HAVE_WASM, loadWasm } from '../net/test-support';

export { HAVE_WASM, loadWasm };

const ROOT = fileURLToPath(new URL('../../../', import.meta.url));
export const CHUNK_DIR = `${ROOT}data/maps/m0/chunks/`;
export const HAVE_TERRAIN = existsSync(`${CHUNK_DIR}c_0_0.bin`);
export const REQUIRE = process.env.FRONTLINE_REQUIRE_TERRAIN === '1';
export const CAN_RUN = HAVE_WASM && HAVE_TERRAIN;
export const TICK_MS = 1000 / 30;

export function loadChunks(): { bx: number; bz: number; data: Uint8Array }[] {
  const out = [];
  for (let bz = 0; bz < 4; bz++) for (let bx = 0; bx < 4; bx++) out.push({ bx, bz, data: new Uint8Array(readFileSync(`${CHUNK_DIR}c_${bx}_${bz}.bin`)) });
  return out;
}

export function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// ---------------------------------------------------------------------------------------------

export interface LinkModel {
  oneWayMs: number;
  /** uniform extra delay in [0, jitterMs] */
  jitterMs: number;
  loss: number;
}

class Link<T> {
  private q: { at: number; item: T }[] = [];
  sent = 0;
  lost = 0;
  constructor(
    private readonly m: LinkModel,
    private readonly rnd: () => number,
  ) {}
  send(now: number, item: T): void {
    this.sent++;
    if (this.rnd() < this.m.loss) {
      this.lost++;
      return;
    }
    this.q.push({ at: now + this.m.oneWayMs + this.rnd() * this.m.jitterMs, item });
  }
  /** Items due at `now`, in arrival order (jitter reorders). */
  take(now: number): T[] {
    if (this.q.length === 0) return [];
    const due = this.q.filter((x) => x.at <= now).sort((a, b) => a.at - b.at);
    if (due.length) this.q = this.q.filter((x) => x.at > now);
    return due.map((x) => x.item);
  }
}

// --- fake snapshot wire --------------------------------------------------------------------

export interface FakeSnap {
  tick: number;
  ackInputSeq: number;
  inputSlack: number;
  self: Float32Array | null;
  ids: number[];
  pos: number[];
  yaw: number[];
  pitch: number[];
  removed: number[];
  events: Uint8Array;
}

export class FakeSnapshotDecoder implements WasmSnapshotDecoder {
  static readonly store = new Map<number, FakeSnap>();
  private cur: FakeSnap | undefined;
  private selfArr = new Float32Array(LAYOUT_SELF_LEN);
  decodeCalls = 0;
  static wire(s: FakeSnap): Uint8Array {
    const id = FakeSnapshotDecoder.store.size + 1;
    FakeSnapshotDecoder.store.set(id, s);
    return Uint8Array.of(id & 255, (id >> 8) & 255, (id >> 16) & 255, 0);
  }
  decode(bytes: Uint8Array): number {
    this.decodeCalls++;
    const id = (bytes[0] as number) | ((bytes[1] as number) << 8) | ((bytes[2] as number) << 16);
    const s = FakeSnapshotDecoder.store.get(id);
    if (!s) throw new Error('unknown fake snapshot');
    this.cur = s;
    this.selfArr.fill(0);
    if (s.self) this.selfArr.set(s.self);
    return s.tick;
  }
  reset(): void {
    this.cur = undefined;
  }
  tick(): number {
    return this.cur?.tick ?? 0;
  }
  ackInputSeq(): number {
    return this.cur?.ackInputSeq ?? 0;
  }
  inputSlack(): number {
    return this.cur?.inputSlack ?? 0;
  }
  count(): number {
    return this.cur?.ids.length ?? 0;
  }
  selfState(): Float32Array {
    return this.selfArr;
  }
  ids(): Uint8Array {
    return Uint8Array.from(this.cur?.ids ?? []);
  }
  pos(): Float32Array {
    return Float32Array.from(this.cur?.pos ?? []);
  }
  yaw(): Float32Array {
    return Float32Array.from(this.cur?.yaw ?? []);
  }
  pitch(): Float32Array {
    return Float32Array.from(this.cur?.pitch ?? []);
  }
  private zeros(): Uint8Array {
    return new Uint8Array(this.cur?.ids.length ?? 0);
  }
  anim(): Uint8Array {
    return this.zeros();
  }
  stance(): Uint8Array {
    return this.zeros();
  }
  weapon(): Uint8Array {
    return this.zeros();
  }
  flags(): Uint8Array {
    return this.zeros();
  }
  health(): Uint8Array {
    return this.zeros();
  }
  removed(): Uint8Array {
    return Uint8Array.from(this.cur?.removed ?? []);
  }
  events(): Uint8Array {
    return this.cur?.events ?? new Uint8Array(0);
  }
  free(): void {
    /* nothing */
  }
}

export function selfArray(o: { corr?: [number, number, number]; ammo?: number; reserve?: number; health?: number }): Float32Array {
  const a = new Float32Array(LAYOUT_SELF_LEN);
  a[0] = 1;
  if (o.corr) {
    a[1] = 1;
    a[2] = o.corr[0];
    a[3] = o.corr[1];
    a[4] = o.corr[2];
  }
  a[5] = o.ammo ?? 0;
  a[6] = o.reserve ?? 0;
  a[7] = o.health ?? 255;
  return a;
}

// --- scripted input --------------------------------------------------------------------------

export type InputScript = (n: number) => Partial<InputCmdFields>;

/** A scripted `InputSource`: `script(stepIndex)` overrides fields of an idle command. */
export class ScriptInput implements InputSource {
  n = 0;
  yawDeg = 0;
  constructor(private readonly script: InputScript = () => ({})) {}
  build(seq: number, tick: number, viewTick: number): InputCmdFields {
    const o = this.script(this.n++);
    return { seq, tick, moveX: 0, moveZ: 0, yaw: 0, pitch: 0, buttons: 0, weaponSlot: 0, aux: 0, viewTick, ...o };
  }
  releaseAll(): void {
    /* nothing */
  }
}

/** Walks forward while turning (a circle of ~30 m radius, stays inside the 256 m map). */
export const walkCircle: InputScript = (n) => ({ moveZ: 127, yaw: Math.round(((n * 0.5 * Math.PI) / 180 / (Math.PI * 2)) * 65536) & 0xffff });
export const standStill: InputScript = () => ({});

// --- the fake server ----------------------------------------------------------------------

interface BundleArgs {
  cmds: Uint8Array;
  ackTick: number;
  predHash: number;
}

const rd = {
  seq: (c: Uint8Array) => ((c[0] as number) << 8) | (c[1] as number),
  tick: (c: Uint8Array) => (((c[2] as number) << 24) | ((c[3] as number) << 16) | ((c[4] as number) << 8) | (c[5] as number)) >>> 0,
};

export interface Scenario {
  seconds: number;
  /** client -> server and server -> client link models */
  up: LinkModel;
  down: LinkModel;
  /** bundles / snapshots dropped by the link ("acceptance 2" model: input never lost) */
  script: InputScript;
  seed?: number;
  /** Server clock estimate error of the client, in ticks (constant offset + slow wobble amplitude). */
  clockOffsetTicks?: number;
  clockWobbleTicks?: number;
  /** Fake wasm perturbation: every `kickEveryS` the server teleports the player by `kick` metres in x. */
  kick?: { everyS: number; metres: number };
  /** Number of scripted remote entities (constant-velocity bots). */
  bots?: number;
  /** Frame times of the client in ms (default 1000/60). */
  frameMs?: number;
  /** Record per-frame samples (render pos) for post analysis. */
  recordFrames?: boolean;
}

export interface ScenarioResult {
  corrections: number;
  snaps: number;
  initialSyncs: number;
  toleranceKept: number;
  maxErrM: number;
  /** Largest render-offset magnitude seen on any frame (the visible error). */
  maxVisibleOffsetM: number;
  /** Largest frame-to-frame jump of the local render position minus its expected motion (m). */
  maxFrameJumpM: number;
  bundles: number;
  bundleBytesAvg: number;
  bundleBytesMax: number;
  bytesByN: number[];
  countByN: number[];
  snapshotsSent: number;
  snapshotsLost: number;
  snapshotsApplied: number;
  bundlesLost: number;
  serverRepeated: number;
  tickExtras: number;
  tickSkips: number;
  ringOverflows: number;
  replayedCommands: number;
  /** Horizontal distance the authoritative player travelled. */
  pathM: number;
  /** wall ms spent inside game.update + onSnapshot per frame (CPU cost of the client side, includes wasm). */
  updateMs: number[];
  /** Server tick of every correction (for "per 10 s" statistics). */
  correctionTimesS: number[];
  game: GameLoop;
}

export function spyWasm(real: SimWasmModule, sink: { last?: BundleArgs }): SimWasmModule {
  return new Proxy(real, {
    get(t, k, r) {
      if (k === 'encodeInputBundle') {
        return (cmds: Uint8Array, ackTick: number, hash: number, fire?: Uint8Array): Uint8Array => {
          sink.last = { cmds: cmds.slice(), ackTick, predHash: hash };
          return real.encodeInputBundle(cmds, ackTick, hash, fire);
        };
      }
      return Reflect.get(t, k, r) as unknown;
    },
  });
}

export async function runScenario(sc: Scenario): Promise<ScenarioResult> {
  const real = await loadWasm();
  const sink: { last?: BundleArgs } = {};
  const wasm = spyWasm(real, sink);
  const rnd = mulberry32(sc.seed ?? 1);
  const chunks = loadChunks();
  const up = new Link<BundleArgs>(sc.up, rnd);
  const down = new Link<Uint8Array>(sc.down, rnd);
  const bus = new EventBus<GameEvents>();
  const dec = new FakeSnapshotDecoder();
  let t = 0; // virtual ms, also the server's clock
  const rtt = sc.up.oneWayMs + sc.down.oneWayMs + (sc.up.jitterMs + sc.down.jitterMs) / 2;
  const wobble = sc.clockWobbleTicks ?? 0;
  const off = sc.clockOffsetTicks ?? 0;
  const nowTicks = (): number => t / TICK_MS + off + wobble * Math.sin(t / 1700);
  const clock: ClockPort = {
    now: nowTicks,
    estimatedServerTick: () => Math.floor(nowTicks()),
    inputTargetTick: () => Math.floor(nowTicks()) + Math.ceil(rtt / 2 / TICK_MS) + 1,
    applySlack: () => undefined,
    rtt: () => rtt,
  };
  const net: NetPort = {
    clock,
    sendInputBundle: (bytes) => {
      const a = sink.last;
      if (!a) throw new Error('bundle without recorded args');
      if ((real as unknown as { inputBundleLen(b: Uint8Array): number }).inputBundleLen(bytes) !== a.cmds.length / 18) throw new Error('bundle length mismatch');
      up.send(t, a);
    },
  };
  const input = new ScriptInput(sc.script);
  const game = new GameLoop({ wasm, decoder: dec, bus, net, input, matchSeed: 1n, mapId: 0, playerId: 3, now: () => t, log: () => undefined });
  game.loadCollision(chunks);

  // authoritative side
  const srv: WasmPredictor = new real.Predictor(1n, 0, 3);
  srv.setEntity(3);
  srv.equipWeapon('us_m1_garand', 80);
  for (const c of chunks) srv.loadCollision(c.bx, c.bz, c.data);
  const spawn = new Uint8Array(39);
  new DataView(spawn.buffer).setFloat32(0, 128, true);
  new DataView(spawn.buffer).setFloat32(4, 2.4, true);
  new DataView(spawn.buffer).setFloat32(8, 56, true);
  srv.reset(spawn, 0);

  const inbox = new Map<number, Uint8Array>();
  let lastCmd: Uint8Array | undefined;
  let lastRealSeq = 0;
  let gap = 0;
  let serverTick = 0;
  let serverRepeated = 0;
  let pathM = 0;
  let lastX = 128;
  let lastZ = 56;
  let reportedHash = 0;
  let reportedAck = 0;
  let haveReport = false;
  const posAtSnap = new Map<number, number>(); // snapshot tick -> hash of authoritative pos
  const snapPos = new Map<number, [number, number, number]>();
  let snapsSent = 0;
  let nextKickT = sc.kick ? sc.kick.everyS * 1000 : Infinity;
  const botCount = sc.bots ?? 0;

  const repack = (c: Uint8Array, clearEdges: boolean, zeroMove: boolean): Uint8Array => {
    const sgn = (v: number): number => (v > 127 ? v - 256 : v);
    const buttons = (((c[12] as number) << 8) | (c[13] as number)) & (clearEdges ? ~(1 | 4 | 8) : 0xffff);
    return real.packInputCmd(rd.seq(c), rd.tick(c), zeroMove ? 0 : sgn(c[6] as number), zeroMove ? 0 : sgn(c[7] as number), ((c[8] as number) << 8) | (c[9] as number), (((c[10] as number) << 8) | (c[11] as number)) << 16 >> 16, buttons, c[14] as number, c[15] as number, ((c[16] as number) << 8) | (c[17] as number));
  };

  const serverTickRun = (): void => {
    const T = serverTick;
    let c = inbox.get(T);
    if (c) {
      gap = 0;
      lastCmd = c;
      lastRealSeq = rd.seq(c);
    } else if (lastCmd) {
      gap++;
      serverRepeated++;
      c = repack(lastCmd, true, gap > 5);
    } else {
      c = real.packInputCmd(0, T, 0, 0, 0, 0, 0, 0, 0, 0);
    }
    srv.step(c);
    {
      const st = srv.state();
      pathM += Math.hypot((st[0] as number) - lastX, (st[2] as number) - lastZ);
      lastX = st[0] as number;
      lastZ = st[2] as number;
    }
    inbox.delete(T - 10);
    if (t >= nextKickT && sc.kick) {
      nextKickT += sc.kick.everyS * 1000;
      const a = srv.authState();
      const dv = new DataView(a.buffer, a.byteOffset, a.byteLength);
      dv.setFloat32(0, dv.getFloat32(0, true) + sc.kick.metres, true);
      srv.reset(a, srv.tick());
    }
    // 20 Hz snapshots: 2 of every 3 ticks
    if (T % 3 !== 2) {
      const a = srv.authState();
      const dv = new DataView(a.buffer, a.byteOffset, a.byteLength);
      const p: [number, number, number] = [dv.getFloat32(0, true), dv.getFloat32(4, true), dv.getFloat32(8, true)];
      snapPos.set(T, p);
      posAtSnap.set(T & 0xffff, predHash(p[0], p[1], p[2]));
      snapPos.delete(T - 200);
      posAtSnap.delete((T - 200) & 0xffff);
      let corr: [number, number, number] | undefined;
      if (!haveReport || posAtSnap.get(reportedAck) !== reportedHash) corr = p;
      const ids: number[] = [];
      const pos: number[] = [];
      const yaw: number[] = [];
      const pitch: number[] = [];
      for (let b = 0; b < botCount; b++) {
        const vx = 2 + (b % 3);
        const bt = (T / 30) * vx;
        ids.push(10 + b);
        pos.push(40 + b * 5 + bt, 2, 100 + b * 3);
        yaw.push(Math.PI / 2);
        pitch.push(0);
      }
      const snap: FakeSnap = { tick: T, ackInputSeq: lastRealSeq, inputSlack: 1, self: selfArray({ ...(corr ? { corr } : {}), ammo: 8, reserve: 80 }), ids, pos, yaw, pitch, removed: [], events: new Uint8Array(0) };
      snapsSent++;
      down.send(t, FakeSnapshotDecoder.wire(snap));
    }
    serverTick++;
  };

  const stepper = new FixedStepper();
  const frameMs = sc.frameMs ?? 1000 / 60;
  const updateMs: number[] = [];
  const correctionTimesS: number[] = [];
  let maxVisible = 0;
  let maxJump = 0;
  let applied = 0;
  bus.on('snapshot.applied', () => applied++);
  let lastCorr = 0;
  let prev: [number, number, number] | undefined;
  const frames = Math.floor((sc.seconds * 1000) / frameMs);
  for (let f = 0; f < frames; f++) {
    t += frameMs;
    // server ticks due
    while ((serverTick + 1) * TICK_MS <= t) serverTickRun();
    for (const a of up.take(t)) {
      reportedHash = a.predHash;
      reportedAck = a.ackTick;
      haveReport = true;
      for (let i = 0; i < a.cmds.length; i += 18) {
        const c = a.cmds.slice(i, i + 18);
        const tk = rd.tick(c);
        if (tk >= serverTick - 2) inbox.set(tk, c); // too old is dropped
      }
    }
    const w0 = performance.now();
    for (const b of down.take(t)) game.onSnapshot(b);
    const { steps, alpha } = stepper.advance(frameMs);
    game.update(steps, alpha, frameMs);
    updateMs.push(performance.now() - w0);
    if (game.corrections !== lastCorr) {
      lastCorr = game.corrections;
      correctionTimesS.push(t / 1000);
    }
    const vis = game.reconciler.smoother.magnitude;
    if (vis > maxVisible) maxVisible = vis;
    const p = game.state.local.pos;
    if (prev && t > 2000) {
      const v = game.state.local.vel;
      const exp = Math.hypot(v[0] as number, v[2] as number) * (frameMs / 1000);
      const jump = Math.max(0, Math.hypot((p[0] as number) - prev[0], (p[2] as number) - prev[2]) - exp - 0.2);
      if (jump > maxJump) maxJump = jump;
    }
    prev = [p[0] as number, p[1] as number, p[2] as number];
  }
  const s = game.stats;
  return {
    corrections: game.reconciler.stats.corrections,
    snaps: game.reconciler.stats.snaps,
    initialSyncs: game.reconciler.stats.initialSyncs,
    toleranceKept: game.reconciler.stats.toleranceKept,
    maxErrM: game.reconciler.stats.maxErrM,
    maxVisibleOffsetM: maxVisible,
    maxFrameJumpM: maxJump,
    bundles: s.bundles,
    bundleBytesAvg: s.bundles ? s.bundleBytes / s.bundles : 0,
    bundleBytesMax: s.bundleBytesMax,
    bytesByN: s.bytesByN.slice(),
    countByN: s.countByN.slice(),
    snapshotsSent: snapsSent,
    snapshotsLost: down.lost,
    snapshotsApplied: applied,
    bundlesLost: up.lost,
    serverRepeated,
    tickExtras: game.planner.extras,
    tickSkips: game.planner.skips,
    ringOverflows: game.ring.overflows,
    replayedCommands: game.reconciler.stats.replayedCommands,
    pathM,
    updateMs,
    correctionTimesS,
    game,
  };
}
