import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { EventBus, type GameEvents } from '../game/event-bus';
import { TeamBalanceCode, TeamMoveFlags, VoteKind } from '../game/state/c4';
import { GameState } from '../game/state/game-state';
import { NetCodec } from './codec';
import { ChatChannel, PingKind } from './comms';
import { connect, type ConnectOptions } from './connect';
import { parseNetConfig, type NetConfig } from './config';
import { disconnectBytes, FakeWebSocket, FakeWebTransport, type FakeServerOpts } from './fakes';
import { NetError } from './handshake';
import { HAVE_WASM, loadWasm, rejectBytes, welcomeBytes } from './test-support';
import type { SimWasmModule } from './wasm';
import type { WebTransportCtor } from './wt-transport';

const CERT = '0a79805c238d6e60bfa1762403734fb652bb3a5977774394be88bf60c7ac25d3';
const cfg = (q = ''): NetConfig => parseNetConfig(`?server=127.0.0.1:7000${q ? '&' + q : ''}`, { hostname: 'localhost', protocol: 'http:' });

describe.skipIf(!HAVE_WASM)('connect / fallback / handshake (fake WebSocket + WebTransport servers, real sim-wasm)', () => {
  let wasm: SimWasmModule;
  let server: FakeServerOpts;
  const wtCtor = FakeWebTransport as unknown as WebTransportCtor;
  let bus: EventBus<GameEvents>;
  let events: string[];

  beforeAll(async () => {
    wasm = await loadWasm();
  });
  beforeEach(() => {
    server = { dataHash: wasm.dataHash(), protocolVersion: wasm.protocolVersion() };
    FakeWebSocket.reset(server);
    FakeWebTransport.reset(server);
    bus = new EventBus<GameEvents>();
    events = [];
    bus.on('net.connected', (e) => events.push(`connected:${e.transport}`));
    bus.on('net.disconnected', (e) => events.push(`disconnected:${e.reason}`));
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  const opts = (over: Partial<ConnectOptions> = {}): ConnectOptions => ({
    config: cfg(),
    wasm,
    bus,
    WebTransportCtor: wtCtor,
    WebSocketCtor: FakeWebSocket,
    ...over,
  });

  it('emits reliable comms notice keys', async () => {
    const seen = vi.fn();
    bus.on('comms.notice', seen);
    const session = await connect(opts({ config: cfg('transport=ws'), teamMarkers: true }));
    try {
      const conn = FakeWebSocket.instances[0]!.conn;
      session.sendPingPlace(PingKind.NeedMedic, [2, 3]);
      await vi.waitFor(() => expect(conn.receivedC0).toContainEqual(Uint8Array.of(0x1e, 2, 0, 2, 0, 3, 0, 0)));
      const key = new TextEncoder().encode('comms.ping.cooldown');
      conn.c0(Uint8Array.of(0x1f, key.length, ...key));
      await vi.waitFor(() => expect(seen).toHaveBeenCalledWith({ key: 'comms.ping.cooldown' }));
    } finally {
      session.close();
    }
  });

  it.each(['ws', 'wt'] as const)('sends chat and delivers the server sender over %s', async (kind) => {
    const busChat = vi.fn();
    const sessionChat = vi.fn();
    bus.on('comms.chat', busChat);
    const session = await connect(opts({ config: cfg(`transport=${kind}`) }));
    const off = session.onChat(sessionChat);
    try {
      const conn = (kind === 'wt' ? FakeWebTransport.instances[0] : FakeWebSocket.instances[0])!.conn;
      const bytes = (value: string) => Uint8Array.from(value.match(/../g)!, (byte) => parseInt(byte, 16));
      session.sendChat(ChatChannel.Team, 'é界😀');
      await vi.waitFor(() => expect(conn.receivedC0).toContainEqual(bytes('13010270ea79e5633c27e6200000')));
      expect(busChat).not.toHaveBeenCalled();
      conn.c0(bytes('13010270ea79e5633c27e6200a80'));
      const expected = { channel: ChatChannel.Team, text: 'é界😀', sender: 42 };
      await vi.waitFor(() => expect(busChat).toHaveBeenCalledWith(expected));
      expect(sessionChat).toHaveBeenCalledWith(expected);
      off();
      conn.c0(bytes('13010270ea79e5633c27e6200a80'));
      await vi.waitFor(() => expect(busChat).toHaveBeenCalledTimes(2));
      expect(sessionChat).toHaveBeenCalledTimes(1);
    } finally {
      off();
      session.close();
    }
  });

  describe('reliable game state', () => {
    function bind() {
      const state = new GameState(bus);
      return {
        state,
        onWelcome: ({ team }: { team: number }) => state.seedTeam(team),
        onC4Event: state.apply.bind(state),
        onDeployResult: state.applyDeployResult.bind(state),
      };
    }

    it.each(['ws', 'wt'] as const)('routes rule requests and applies the authoritative team move over %s', async (kind) => {
      const { state, onWelcome, onC4Event, onDeployResult } = bind();
      const move = vi.fn();
      bus.on('team.changed', move);
      const s = await connect(opts({ config: cfg(`transport=${kind}`), onWelcome, onC4Event, onDeployResult }));
      try {
        const codec = new NetCodec(wasm);
        const conn = (kind === 'wt' ? FakeWebTransport.instances[0] : FakeWebSocket.instances[0])!.conn;
        expect(state.team).toBe(0);
        const voteId = 0x89012345;
        for (const packet of [codec.encodeVoteStart(VoteKind.Kick, 37), codec.encodeVoteCast(voteId, false),
          codec.encodeTeamSwitchRequest(1, false), codec.encodeForgivePunish(0xbeef, true), codec.encodeMapVoteCast(voteId, 2)]) {
          s.sendC0(packet);
        }
        await vi.waitFor(() => expect(conn.receivedC0.filter((b) => b[0]! >= 0x19).map((b) => [...b])).toEqual([
          [0x19, 0, 37], [0x1a, 0x89, 1, 0x23, 0x45, 0], [0x1b, 1, 0], [0x1c, 0xbe, 0xef, 0x80], [0x1d, 0x89, 1, 0x23, 0x45, 2],
        ]));
        state.apply({ type: 'spawnpoint', id: 9, kind: 0, team: 0, pos: [0, 0], state: 1 });
        state.applyDeploy({ type: 'sync', phase: 'Spawned' });
        conn.c4(Uint8Array.of(0x0d, TeamBalanceCode.Moved, 0, 0, TeamMoveFlags.Forced, 1, 0, 0, 0, 40));
        await vi.waitFor(() => expect(move).toHaveBeenCalledExactlyOnceWith({ team: 1, flags: TeamMoveFlags.Forced, tick: 40 }));
        expect(state.team).toBe(1);
        expect(state.deploy.phase).toBe('Idle');
        expect(state.spawnPoints.size).toBe(0);
      } finally {
        s.close();
      }
    });

    it.each(['ws', 'wt'] as const)('applies C0 deploy results and C4 events over %s', async (kind) => {
      const { state, onC4Event, onDeployResult } = bind();
      const deploy = vi.fn();
      const kill = vi.fn();
      bus.on('deploy.state', deploy);
      bus.on('kill.feed', kill);
      const s = await connect(opts({ config: cfg(`transport=${kind}`), onC4Event, onDeployResult }));
      try {
        const conn = (kind === 'wt' ? FakeWebTransport.instances[0] : FakeWebSocket.instances[0])!.conn;
        // DeployResult: type, then ok:1, spawn_tick:32, reason:8, zero padding.
        const okAtTickZero = Uint8Array.of(0x11, 0x80, 0, 0, 0, 0, 0);
        conn.c0(okAtTickZero); // No pending request: must be ignored.
        await vi.waitFor(() => expect(state.deploy.illegal).toBe(1));
        expect(deploy).not.toHaveBeenCalled();
        state.applyDeploy({ type: 'open' });
        state.applyDeploy({ type: 'request' });
        conn.c0(Uint8Array.of(0x11, 0, 0, 0, 0, 21 >>> 1, (21 & 1) << 7));
        await vi.waitFor(() => expect(deploy).toHaveBeenLastCalledWith({ phase: 'Choosing', reason: 'deploy.reject.point_blocked', spawnTick: undefined }));
        state.applyDeploy({ type: 'request' });
        conn.c0(okAtTickZero);
        await vi.waitFor(() => expect(deploy).toHaveBeenLastCalledWith({ phase: 'Counting', spawnTick: 0, reason: undefined }));

        conn.c4(Uint8Array.of(0xff)); // Invalid type; later messages still arrive.
        // Kill: headshot=1, dist_q=400, followed by two zero assists.
        conn.c4(Uint8Array.of(1, 3, 4, 2, 0x8c, 0x80, 0, 0));
        conn.c4(Uint8Array.of(7, 4, 1, 65, 0, 0, 1, 0)); // roster: A
        conn.c4(Uint8Array.of(7, 4, 1, 66, 0, 0, 1, 0)); // roster: B replaces A
        await vi.waitFor(() => expect(state.roster.get(4)?.name).toBe('B'));
        const entry = { killer: 4, victim: 3, weapon: 2, headshot: true, dist: 100 };
        expect(state.killFeed.at(0)).toEqual(entry);
        expect(kill.mock.calls).toEqual([[entry]]);

        s.close();
        conn.c4(Uint8Array.of(1, 3, 4, 2, 0x8c, 0x80, 0, 0));
        await new Promise<void>((resolve) => setTimeout(resolve, 0));
        expect(kill).toHaveBeenCalledTimes(1);
      } finally {
        s.close();
      }
    });

    it.each(['welcome', 'reject'] as const)('WT buffers early C4 and only publishes after Welcome (%s)', async (answer) => {
      const { state, onWelcome, onC4Event } = bind();
      const objective = vi.fn();
      bus.on('objective.changed', objective);
      FakeWebTransport.reset({ ...server, silent: true });
      const pending = connect(opts({ config: cfg('transport=wt'), onWelcome, onC4Event }));
      const rejection = answer === 'reject' ? expect(pending).rejects.toMatchObject({ code: 'rejected' }) : undefined;
      await vi.waitFor(() => expect(FakeWebTransport.instances[0]?.conn.receivedC0[0]?.[0]).toBe(1));
      const { conn } = FakeWebTransport.instances[0]!;
      conn.c4(Uint8Array.of(5, 2, 1, 0));
      conn.c4(Uint8Array.of(5, 2, 0, 0));
      conn.c4(Uint8Array.of(0x0d, TeamBalanceCode.Moved, 0, 0, TeamMoveFlags.Forced, 1, 0, 0, 0, 40));
      await new Promise<void>((resolve) => setTimeout(resolve, 0));
      expect(objective).not.toHaveBeenCalled();
      if (answer === 'reject') {
        conn.c0(rejectBytes(1));
        await rejection;
        expect(state.objectives.size).toBe(0);
        expect(state.team).toBeUndefined();
      } else {
        conn.c0(welcomeBytes({ dataHash: server.dataHash }));
        const s = await pending;
        try {
          expect(objective.mock.calls).toEqual([[{ objective: 2, owner: 1, reason: 0 }], [{ objective: 2, owner: 0, reason: 0 }]]);
          expect(state.objectives.get(2)?.owner).toBe(0);
          expect(s.welcome.team).toBe(0);
          expect(state.team).toBe(1);
        } finally {
          s.close();
        }
      }
    });
  });

  describe('fallback', () => {
    it('WebTransport undefined: WebTransport is never tried, WebSocket session', async () => {
      const s = await connect(opts({ WebTransportCtor: null }));
      expect(s.kind).toBe('ws');
      expect(FakeWebTransport.instances).toHaveLength(0);
      expect(s.fallbackReason).toBe('WebTransport undefined');
      expect(FakeWebSocket.instances[0]?.url).toBe('ws://127.0.0.1:7000/');
      s.close();
    });

    it('ready never resolves: falls back after 3 s, total <= 5 s', async () => {
      vi.useFakeTimers();
      FakeWebTransport.mode = 'never-ready';
      const t0 = Date.now();
      const p = connect(opts({ now: () => Date.now() }));
      await vi.advanceTimersByTimeAsync(2900);
      expect(FakeWebSocket.instances).toHaveLength(0); // still waiting for WT
      await vi.advanceTimersByTimeAsync(200);
      await vi.advanceTimersByTimeAsync(50);
      const s = await p;
      expect(s.kind).toBe('ws');
      const total = Date.now() - t0;
      expect(total).toBeGreaterThanOrEqual(3000);
      expect(total).toBeLessThanOrEqual(5000);
      expect(s.connectMs).toBeLessThanOrEqual(5000);
      expect(FakeWebTransport.instances[0]?.closedByClient).toBe(true); // the stuck session was abandoned
      expect(s.fallbackReason).toMatch(/not ready after 3000 ms/);
      s.close();
    });

    it('ready rejects: immediate fallback (no 3 s wait)', async () => {
      FakeWebTransport.mode = 'reject-ready';
      const t0 = performance.now();
      const s = await connect(opts());
      expect(s.kind).toBe('ws');
      expect(performance.now() - t0).toBeLessThan(1000);
      expect(s.fallbackReason).toMatch(/connection refused/);
      s.close();
    });

    it('WebTransport constructor throws: fallback', async () => {
      FakeWebTransport.mode = 'throw';
      const s = await connect(opts());
      expect(s.kind).toBe('ws');
      expect(s.fallbackReason).toMatch(/constructor rejects/);
      s.close();
    });

    it('WT session ready but the handshake never completes: falls back within the 3 s budget', async () => {
      vi.useFakeTimers();
      FakeWebTransport.mode = 'no-answer';
      const p = connect(opts({ now: () => Date.now() }));
      await vi.advanceTimersByTimeAsync(3100);
      await vi.advanceTimersByTimeAsync(50);
      const s = await p;
      expect(s.kind).toBe('ws');
      expect(s.fallbackReason).toMatch(/No answer from the server/);
      s.close();
    });

    it('?transport=ws forces WebSocket even when WebTransport works', async () => {
      const s = await connect(opts({ config: cfg('transport=ws') }));
      expect(s.kind).toBe('ws');
      expect(FakeWebTransport.instances).toHaveLength(0);
      expect(s.fallbackReason).toBeUndefined();
      s.close();
    });

    it('?transport=wt does not fall back', async () => {
      FakeWebTransport.mode = 'reject-ready';
      await expect(connect(opts({ config: cfg('transport=wt') }))).rejects.toMatchObject({ code: 'transport_failed' });
      expect(FakeWebSocket.instances).toHaveLength(0);
      await expect(connect(opts({ config: cfg('transport=wt'), WebTransportCtor: null }))).rejects.toThrow(/not available/);
    });

    it('both paths dead: the WebSocket error is reported', async () => {
      FakeWebTransport.mode = 'reject-ready';
      FakeWebSocket.failConnect = true;
      await expect(connect(opts())).rejects.toMatchObject({ code: 'transport_failed' });
    });
  });

  describe('WebTransport path', () => {
    it('connects, pins the certificate only when certhash is given, handshake over the C0 stream', async () => {
      const s = await connect(opts({ config: cfg(`certhash=${CERT}`) }));
      expect(s.kind).toBe('wt');
      const wt = FakeWebTransport.instances[0] as FakeWebTransport;
      expect(wt.url).toBe('https://127.0.0.1:7001/wt');
      expect(wt.options?.serverCertificateHashes?.[0]?.algorithm).toBe('sha-256');
      expect([...(wt.options?.serverCertificateHashes?.[0]?.value ?? [])].map((b) => b.toString(16).padStart(2, '0')).join('')).toBe(CERT);
      expect(s.welcome).toMatchObject({ playerId: 7, tickHz: 30, matchSeed: 0x1234_5678_9abcn });
      expect(s.fallbackReason).toBeUndefined();
      expect(s.transport.maxDatagramSize).toBe(1199);
      expect(events).toEqual(['connected:wt']);
      // Hello carries caps bit1 (webtransport)
      const hello = wt.conn.receivedC0[0] as Uint8Array;
      expect([...hello]).toEqual([...wasm.encodeHello(server.protocolVersion, server.dataHash, new Uint8Array(0), 0, 2)]);
      s.close();
      const plain = await connect(opts());
      expect(FakeWebTransport.instances[1]?.options).toBeUndefined();
      plain.close();
    });

    it('routes C2 datagrams and C4 stream frames to the callbacks (raw bytes) and sends C1 as datagrams', async () => {
      const s = await connect(opts());
      const wt = FakeWebTransport.instances[0] as FakeWebTransport;
      const snaps: number[][] = [];
      const c4: number[][] = [];
      s.onSnapshot((b) => snaps.push([...b]));
      s.onC4((b) => c4.push([...b]));
      wt.conn.c2(Uint8Array.of(1, 2, 3));
      wt.conn.c4(Uint8Array.of(9, 9));
      wt.conn.c4(Uint8Array.of(8));
      await vi.waitFor(() => expect(c4).toEqual([[9, 9], [8]]));
      expect(snaps).toEqual([[1, 2, 3]]);
      s.sendInputBundle(Uint8Array.of(5, 6, 7));
      s.sendInputBundle(new Uint8Array(2000)); // above the datagram budget: dropped, not thrown
      await vi.waitFor(() => expect(wt.conn.receivedC1).toHaveLength(1));
      expect([...(wt.conn.receivedC1[0] as Uint8Array)]).toEqual([5, 6, 7]);
      expect(s.stats().transport).toBe('wt');
      expect(s.stats().pktsOut).toBeGreaterThan(0);
      s.close();
    });
  });

  describe('WebSocket path', () => {
    it('multiplexed frames; Welcome fields; stats', async () => {
      const s = await connect(opts({ WebTransportCtor: null, config: cfg('transport=ws') }));
      const ws = FakeWebSocket.instances[0] as FakeWebSocket;
      expect(ws.binaryType).toBe('arraybuffer');
      expect(s.welcome.tickHz).toBe(30);
      expect(s.welcome.matchSeed).not.toBe(0n);
      const got: number[][] = [];
      s.onSnapshot((b) => got.push([...b]));
      ws.conn.c2(Uint8Array.of(4, 5));
      expect(got).toEqual([[4, 5]]);
      s.sendInputBundle(Uint8Array.of(7));
      expect([...(ws.conn.receivedC1[0] ?? [])]).toEqual([7]);
      expect(s.stats()).toMatchObject({ transport: 'ws' });
      expect(s.stats().bytesIn).toBeGreaterThan(0);
      expect(events).toEqual(['connected:ws']);
      s.close();
    });

    it('C1 under congestion (bufferedAmount > 8 KB): only the newest packet survives and is sent once the buffer drains', async () => {
      const s = await connect(opts({ config: cfg('transport=ws') }));
      const ws = FakeWebSocket.instances[0] as FakeWebSocket;
      FakeWebSocket.buffered = 20_000;
      for (let i = 1; i <= 5; i++) s.sendInputBundle(Uint8Array.of(i));
      expect(ws.conn.receivedC1).toHaveLength(0);
      FakeWebSocket.buffered = 0;
      await vi.waitFor(() => expect(ws.conn.receivedC1).toHaveLength(1));
      expect([...(ws.conn.receivedC1[0] as Uint8Array)]).toEqual([5]);
      // a fresh packet while idle goes straight out
      s.sendInputBundle(Uint8Array.of(6));
      expect(ws.conn.receivedC1).toHaveLength(2);
      s.close();
    });

    it('malformed frames from the wire are ignored, not thrown', async () => {
      const s = await connect(opts({ config: cfg('transport=ws') }));
      const ws = FakeWebSocket.instances[0] as FakeWebSocket;
      expect(() => ws.onmessage?.({ data: new Uint8Array([9, 0, 0]).buffer })).not.toThrow();
      expect(() => ws.onmessage?.({ data: 'text' })).not.toThrow();
      s.close();
    });
  });

  describe('handshake failures are reported, not silent', () => {
    it('protocol_version mismatch -> Reject(version_mismatch) with a readable message', async () => {
      const err = await connect(opts({ config: cfg('transport=ws'), hello: { protocolVersion: 4 } })).catch((e: unknown) => e);
      expect(err).toBeInstanceOf(NetError);
      expect(err).toMatchObject({ code: 'rejected', reason: 'version_mismatch' });
      expect((err as Error).message).toMatch(/rejected.*version_mismatch.*protocol versions differ/i);
      expect(events).toEqual([]);
    });

    it('data_hash mismatch -> Reject(data_hash_mismatch)', async () => {
      const err = await connect(opts({ config: cfg('transport=ws'), hello: { dataHash: server.dataHash ^ 1n } })).catch((e: unknown) => e);
      expect(err).toMatchObject({ code: 'rejected', reason: 'data_hash_mismatch' });
      expect((err as Error).message).toMatch(/game data differ/);
    });

    it('the same Reject over WebTransport is final: no silent fallback to WebSocket', async () => {
      const err = await connect(opts({ hello: { protocolVersion: 99 } })).catch((e: unknown) => e);
      expect(err).toMatchObject({ code: 'rejected', reason: 'version_mismatch' });
      expect(FakeWebSocket.instances).toHaveLength(0);
    });

    it('a Welcome whose data_hash differs from the local data aborts (server that does not reject)', async () => {
      const { handshake } = await import('./handshake');
      const { NetCodec } = await import('./codec');
      const { welcomeBytes } = await import('./test-support');
      const c0: ((b: Uint8Array) => void)[] = [];
      const fakeT = {
        kind: 'ws' as const,
        maxDatagramSize: 1,
        counters: { bytesIn: 0, bytesOut: 0, pktsIn: 0, pktsOut: 0 },
        connect: () => Promise.resolve(),
        sendC0: () => queueMicrotask(() => c0.forEach((f) => f(welcomeBytes({ dataHash: server.dataHash + 1n })))),
        sendC1: () => undefined,
        onC0: (f: (b: Uint8Array) => void) => (c0.push(f), () => undefined),
        onC2: () => () => undefined,
        onC3: () => () => undefined,
        onC4: () => () => undefined,
        onClose: () => () => undefined,
        close: () => undefined,
      };
      const err = await handshake(fakeT, new NetCodec(wasm)).catch((e: unknown) => e);
      expect(err).toMatchObject({ code: 'data_hash_mismatch' });
      expect((err as Error).message).toMatch(/Game data mismatch/);
    });

    it('no answer: handshake timeout error', async () => {
      FakeWebSocket.server = { ...server, silent: true };
      const err = await connect(opts({ config: cfg('transport=ws'), handshakeTimeoutMs: 150 })).catch((e: unknown) => e);
      expect(err).toMatchObject({ code: 'handshake_timeout' });
    });
  });

  describe('disconnect and manual reconnect', () => {
    it('server drop: net.disconnected with the reason, onDisconnect fires once, pings stop; connecting again works', async () => {
      const s = await connect(opts({ config: cfg('transport=ws') }));
      const reasons: string[] = [];
      s.onDisconnect((r) => reasons.push(r));
      const ws = FakeWebSocket.instances[0] as FakeWebSocket;
      ws.conn.drop('server restart');
      expect(reasons).toEqual(['ws closed: server restart']);
      expect(events).toEqual(['connected:ws', 'disconnected:ws closed: server restart']);
      s.sendInputBundle(Uint8Array.of(1)); // no throw after the disconnect
      const s2 = await connect(opts({ config: cfg('transport=ws') }));
      expect(s2.welcome.playerId).toBe(7);
      expect(events.at(-1)).toBe('connected:ws');
      s2.close();
      expect(reasons).toHaveLength(1);
    });

    it('server Disconnect message and WebTransport session close are reported', async () => {
      const s = await connect(opts({ config: cfg('transport=ws') }));
      (FakeWebSocket.instances[0] as FakeWebSocket).conn.c0(disconnectBytes(3));
      expect(events.at(-1)).toBe('disconnected:server disconnect: server_shutdown');
      const w = await connect(opts());
      (FakeWebTransport.instances.at(-1) as FakeWebTransport).conn.drop('going away');
      await vi.waitFor(() => expect(events.at(-1)).toBe('disconnected:webtransport closed: going away'));
      void s;
      void w;
    });

    it('close() by the client is not reported as a disconnect', async () => {
      const s = await connect(opts({ config: cfg('transport=ws') }));
      s.close();
      await new Promise((r) => setTimeout(r, 10));
      expect(events).toEqual(['connected:ws']);
    });
  });

  describe('clock integration', () => {
    it('Pongs from the server feed the clock; the estimate follows the server tick', async () => {
      let tick = 1000;
      server.tickAt = () => tick;
      FakeWebSocket.server = server;
      const s = await connect(opts({ config: cfg('transport=ws') }));
      expect(s.clock.synced).toBe(true);
      expect(s.clock.estimatedServerTick()).toBeGreaterThanOrEqual(1000);
      await vi.waitFor(() => expect(s.clock.jitter()).toBeGreaterThanOrEqual(0));
      const before = s.stats().pktsIn;
      await new Promise((r) => setTimeout(r, 450)); // the 5 quick pings come back as Pongs
      expect(s.stats().pktsIn).toBeGreaterThanOrEqual(before + 4);
      tick = 1010;
      await vi.waitFor(() => expect(s.clock.estimatedServerTick()).toBeGreaterThanOrEqual(1010), { timeout: 1500 });
      s.close();
    });
  });
});
