/**
 * Connection set-up: WebTransport first, WebSocket as fallback, then handshake and clock sync.
 *
 * Fallback (`transport=auto`) to WebSocket happens when WebTransport
 *   - is not defined in this browser, or its constructor throws,
 *   - `ready` is rejected (refused, bad certificate / hash, UDP blocked, ...),
 *   - or the WT session + handshake is not complete within 3 s (WT_TIMEOUT_MS),
 *   - or the WT session dies during the handshake.
 * It does NOT happen for a server Reject or a data-hash mismatch: those are authoritative answers
 * and would repeat identically over WebSocket. `transport=ws` never tries WebTransport; `transport=wt`
 * never falls back.
 */
import { EventBus, type GameEvents } from '../game/event-bus';
import { ClockSync, TICK_MS } from './clock';
import { NetCodec, type WelcomeMsg } from './codec';
import { wsUrl, wtUrl, type NetConfig } from './config';
import { handshake, NetError, type HandshakeOptions } from './handshake';
import type { Transport, TransportKind } from './transport';
import type { SimWasmModule } from './wasm';
import { WsTransport, type WebSocketCtor } from './ws-transport';
import { WtTransport, type WebTransportCtor } from './wt-transport';

export const WT_TIMEOUT_MS = 3000;
export const TOTAL_BUDGET_MS = 5000;

export interface NetStats {
  bytesIn: number;
  bytesOut: number;
  pktsIn: number;
  pktsOut: number;
  rttMs: number;
  jitterMs: number;
  transport: TransportKind;
}

export interface ConnectOptions {
  config: NetConfig;
  wasm: SimWasmModule;
  bus?: EventBus<GameEvents>;
  /** Handshake overrides (tests: wrong protocol version / data hash). */
  hello?: Pick<HandshakeOptions, 'protocolVersion' | 'dataHash' | 'clientBuild' | 'ticket'>;
  /** Injection points for tests; default to the browser globals. */
  WebTransportCtor?: WebTransportCtor | null;
  WebSocketCtor?: WebSocketCtor;
  wtTimeoutMs?: number;
  handshakeTimeoutMs?: number;
  now?: () => number;
  /** Progress notes ('wt: skipped (WebTransport undefined)', ...) for diagnostics/tests. */
  log?: (msg: string) => void;
}

export interface Session {
  readonly transport: Transport;
  readonly kind: TransportKind;
  readonly clock: ClockSync;
  readonly welcome: WelcomeMsg;
  /** Handshake round trip (Hello -> Welcome) in ms. */
  readonly handshakeRttMs: number;
  /** Time from the start of `connect()` to a completed handshake, ms. */
  readonly connectMs: number;
  /** Why WebTransport was not used (undefined when it was, or `transport=ws`). */
  readonly fallbackReason: string | undefined;
  /** C2 packets, raw (decode in `game`). */
  onSnapshot(cb: (bytes: Uint8Array) => void): () => void;
  onC3(cb: (bytes: Uint8Array) => void): () => void;
  onC4(cb: (bytes: Uint8Array) => void): () => void;
  /** Fires once when the connection ends for any reason except `close()`. */
  onDisconnect(cb: (reason: string) => void): () => void;
  sendInputBundle(bytes: Uint8Array): void;
  stats(): NetStats;
  close(): void;
}

function describe(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

class SessionImpl implements Session {
  private readonly disconnectCbs: ((reason: string) => void)[] = [];
  private disconnected = false;
  private readonly unsubs: (() => void)[] = [];

  constructor(
    readonly transport: Transport,
    readonly clock: ClockSync,
    readonly welcome: WelcomeMsg,
    readonly handshakeRttMs: number,
    readonly connectMs: number,
    readonly fallbackReason: string | undefined,
    codec: NetCodec,
    private readonly bus: EventBus<GameEvents> | undefined,
    now: () => number,
  ) {
    // C0 after the handshake: Pong feeds the clock, Disconnect ends the session.
    this.unsubs.push(
      transport.onC0((bytes) => {
        let m;
        try {
          m = codec.decodeServerC0(bytes);
        } catch {
          return;
        }
        if (m.kind === 'pong') clock.onPong(m.clientTime, m.serverTick, now());
        else if (m.kind === 'disconnect') this.fail(`server disconnect: ${m.reasonName}`);
      }),
      transport.onClose((why) => this.fail(why)),
    );
  }

  private fail(reason: string): void {
    if (this.disconnected) return;
    this.disconnected = true;
    this.clock.stop();
    for (const u of this.unsubs) u();
    this.transport.close();
    this.bus?.emit('net.disconnected', { reason });
    for (const cb of this.disconnectCbs) cb(reason);
  }

  onSnapshot(cb: (bytes: Uint8Array) => void): () => void {
    return this.transport.onC2(cb);
  }
  onC3(cb: (bytes: Uint8Array) => void): () => void {
    return this.transport.onC3(cb);
  }
  onC4(cb: (bytes: Uint8Array) => void): () => void {
    return this.transport.onC4(cb);
  }
  onDisconnect(cb: (reason: string) => void): () => void {
    this.disconnectCbs.push(cb);
    return () => {
      const i = this.disconnectCbs.indexOf(cb);
      if (i >= 0) this.disconnectCbs.splice(i, 1);
    };
  }

  sendInputBundle(bytes: Uint8Array): void {
    if (!this.disconnected) this.transport.sendC1(bytes);
  }

  stats(): NetStats {
    const c = this.transport.counters;
    return {
      bytesIn: c.bytesIn,
      bytesOut: c.bytesOut,
      pktsIn: c.pktsIn,
      pktsOut: c.pktsOut,
      rttMs: this.clock.rtt(),
      jitterMs: this.clock.jitter(),
      transport: this.kind,
    };
  }

  get kind(): TransportKind {
    return this.transport.kind;
  }

  close(): void {
    if (this.disconnected) return;
    this.disconnected = true;
    this.clock.stop();
    for (const u of this.unsubs) u();
    this.transport.close();
  }
}

/** Races `p` against a timeout; the loser's late rejection is swallowed. */
function withTimeout<T>(p: Promise<T>, ms: number, onTimeout: () => Error): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const t = setTimeout(() => reject(onTimeout()), ms);
    p.then(
      (v) => {
        clearTimeout(t);
        resolve(v);
      },
      (e: unknown) => {
        clearTimeout(t);
        reject(e instanceof Error ? e : new Error(String(e)));
      },
    );
  });
}

export async function connect(opts: ConnectOptions): Promise<Session> {
  const now = opts.now ?? (() => performance.now());
  const log = opts.log ?? (() => undefined);
  const t0 = now();
  const codec = new NetCodec(opts.wasm);
  const cfg = opts.config;
  const wtCtor = opts.WebTransportCtor === undefined ? (globalThis as { WebTransport?: WebTransportCtor }).WebTransport : (opts.WebTransportCtor ?? undefined);
  const hasGpu = typeof navigator !== 'undefined' && 'gpu' in navigator;

  let fallbackReason: string | undefined;
  const attempt = async (t: Transport, connectTimeoutMs: number, hsTimeoutMs: number): Promise<Session> => {
    const caps = (hasGpu ? 1 : 0) | (t.kind === 'wt' ? 2 : 0);
    try {
      await withTimeout(t.connect(), connectTimeoutMs, () => new NetError('transport_failed', `${t.kind} connection not ready after ${connectTimeoutMs} ms`));
    } catch (e) {
      t.close();
      throw e instanceof NetError ? e : new NetError('transport_failed', `${t.kind} connection failed: ${describe(e)}`);
    }
    let hs;
    try {
      hs = await handshake(t, codec, { ...opts.hello, caps, timeoutMs: hsTimeoutMs, now });
    } catch (e) {
      t.close();
      throw e;
    }
    const clock = new ClockSync({ now, sendPing: (ct) => t.sendC0(codec.encodePing(ct)), tickMs: hs.welcome.tickHz > 0 ? 1000 / hs.welcome.tickHz : TICK_MS });
    clock.seed(hs.welcome.serverTick, hs.rttMs, hs.recvAt);
    const s = new SessionImpl(t, clock, hs.welcome, hs.rttMs, now() - t0, fallbackReason, codec, opts.bus, now);
    clock.start();
    opts.bus?.emit('net.connected', { transport: t.kind, rtt: hs.rttMs });
    return s;
  };

  const tryWs = (): Promise<Session> => {
    const remaining = Math.max(1000, TOTAL_BUDGET_MS - (now() - t0));
    log(`ws: connecting ${wsUrl(cfg)}`);
    return attempt(new WsTransport(wsUrl(cfg), opts.WebSocketCtor), remaining, opts.handshakeTimeoutMs ?? remaining);
  };

  if (cfg.transport === 'ws') return tryWs();

  const wantedWt = cfg.transport === 'wt';
  if (!wtCtor) {
    if (wantedWt) throw new NetError('transport_failed', 'WebTransport is not available in this browser');
    fallbackReason = 'WebTransport undefined';
    log(`wt: skipped (${fallbackReason})`);
    return tryWs();
  }

  const wtMs = opts.wtTimeoutMs ?? WT_TIMEOUT_MS;
  const wtStart = now();
  try {
    log(`wt: connecting ${wtUrl(cfg)}${cfg.certHash ? ' (pinned certificate)' : ''}`);
    // One 3 s budget for the WT session to become ready *and* the handshake to complete.
    const wt = new WtTransport(wtUrl(cfg), cfg.certHash, wtCtor);
    return await attempt(wt, wtMs, Math.max(100, wtMs - (now() - wtStart)));
  } catch (e) {
    // Authoritative answers are final; everything else is a WebTransport problem.
    if (e instanceof NetError && (e.code === 'rejected' || e.code === 'data_hash_mismatch')) throw e;
    if (wantedWt) throw e;
    fallbackReason = describe(e);
    log(`wt: failed (${fallbackReason}); falling back to ws`);
    return tryWs();
  }
}
