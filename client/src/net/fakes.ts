// In-memory WebSocket / WebTransport servers for the connect/fallback tests (vitest only).
// They speak the real wire formats (codec.ts) and answer Hello/Ping like crates/server does.
import { Channel, decodeDatagram, decodeWsFrame, encodeDatagram, encodeStreamFrame, encodeWsFrame, StreamFrameReader } from './codec';
import { disconnectBytes, pongBytes, rejectBytes, welcomeBytes } from './test-support';
import type { WebSocketLike } from './ws-transport';
import type { WebTransportLike } from './wt-transport';

export interface FakeServerOpts {
  dataHash: bigint;
  protocolVersion: number;
  tickHz?: number;
  /** Simulated server tick (continuous) as a function of wall time ms; default 0 at t=0. */
  tickAt?: () => number;
  /** Do not answer Hello (handshake timeout tests). */
  silent?: boolean;
}

export interface FakeConn {
  /** Server -> client C0 payload. */
  c0(bytes: Uint8Array): void;
  /** Server -> client C2 payload (datagram / ws frame). */
  c2(bytes: Uint8Array): void;
  c4(bytes: Uint8Array): void;
  /** Server drops the connection. */
  drop(reason?: string): void;
  receivedC1: Uint8Array[];
  receivedC0: Uint8Array[];
}

/** Server logic shared by both transports: returns the C0 answer for a client C0 payload. */
export function answerC0(o: FakeServerOpts, req: Uint8Array): Uint8Array | undefined {
  if (o.silent) return undefined;
  if (req[0] === 1) {
    const version = ((req[1] as number) << 8) | (req[2] as number);
    let hash = 0n;
    for (let i = 0; i < 8; i++) hash = (hash << 8n) | BigInt(req[3 + i] as number);
    if (version !== o.protocolVersion) return rejectBytes(1);
    if (hash !== o.dataHash) return rejectBytes(5);
    return welcomeBytes({ dataHash: o.dataHash, tickHz: o.tickHz ?? 30, serverTick: Math.floor(o.tickAt?.() ?? 0), playerId: 7 });
  }
  if (req[0] === 4) {
    const ct = (((req[1] as number) << 24) | ((req[2] as number) << 16) | ((req[3] as number) << 8) | (req[4] as number)) >>> 0;
    return pongBytes(ct, Math.floor(o.tickAt?.() ?? 0), 1);
  }
  return undefined;
}

export { disconnectBytes };

// --- WebSocket -------------------------------------------------------------------------------

export class FakeWebSocket implements WebSocketLike {
  static server: FakeServerOpts;
  static instances: FakeWebSocket[] = [];
  /** `bufferedAmount` reported to the transport (congestion tests). */
  static buffered = 0;
  static failConnect = false;
  binaryType = 'blob';
  readyState = 0;
  onopen: ((ev: unknown) => void) | null = null;
  onmessage: ((ev: { data: unknown }) => void) | null = null;
  onerror: ((ev: unknown) => void) | null = null;
  onclose: ((ev: { code?: number; reason?: string }) => void) | null = null;
  readonly sent: Uint8Array[] = [];
  readonly conn: FakeConn;

  constructor(readonly url: string) {
    FakeWebSocket.instances.push(this);
    this.conn = {
      receivedC0: [],
      receivedC1: [],
      c0: (b) => this.deliver(Channel.C0, b),
      c2: (b) => this.deliver(Channel.C2, b),
      c4: (b) => this.deliver(Channel.C4, b),
      drop: (reason) => {
        this.readyState = 3;
        this.onclose?.({ code: 1006, ...(reason ? { reason } : {}) });
      },
    };
    setTimeout(() => {
      if (FakeWebSocket.failConnect) {
        this.readyState = 3;
        this.onerror?.({});
        this.onclose?.({ code: 1006 });
        return;
      }
      this.readyState = 1;
      this.onopen?.({});
    }, 0);
  }

  get bufferedAmount(): number {
    return FakeWebSocket.buffered;
  }

  private deliver(ch: Channel, payload: Uint8Array): void {
    const f = encodeWsFrame(ch, payload);
    this.onmessage?.({ data: f.buffer.slice(f.byteOffset, f.byteOffset + f.byteLength) });
  }

  send(data: Uint8Array): void {
    this.sent.push(data);
    const { channel, payload } = decodeWsFrame(data);
    if (channel === Channel.C1) this.conn.receivedC1.push(payload.slice());
    if (channel === Channel.C0) {
      this.conn.receivedC0.push(payload.slice());
      const a = answerC0(FakeWebSocket.server, payload);
      if (a) setTimeout(() => this.deliver(Channel.C0, a), 0);
    }
  }

  close(): void {
    if (this.readyState === 3) return;
    this.readyState = 3;
    setTimeout(() => this.onclose?.({ code: 1000 }), 0);
  }

  static reset(server: FakeServerOpts): void {
    FakeWebSocket.server = server;
    FakeWebSocket.instances = [];
    FakeWebSocket.buffered = 0;
    FakeWebSocket.failConnect = false;
  }
}

// --- WebTransport ----------------------------------------------------------------------------

export type FakeWtMode = 'ok' | 'never-ready' | 'reject-ready' | 'throw' | 'no-answer';

export class FakeWebTransport implements WebTransportLike {
  static server: FakeServerOpts;
  static mode: FakeWtMode = 'ok';
  static instances: FakeWebTransport[] = [];
  static maxDatagram = 1200;

  readonly ready: Promise<unknown>;
  readonly closed: Promise<unknown>;
  readonly datagrams: WebTransportLike['datagrams'];
  readonly incomingUnidirectionalStreams: WebTransportLike['incomingUnidirectionalStreams'];
  readonly conn: FakeConn;
  closedByClient = false;

  private resolveClosed!: (v: unknown) => void;
  private dgCtl!: ReadableStreamDefaultController<Uint8Array>;
  private bidiCtl!: ReadableStreamDefaultController<Uint8Array>;
  private uniCtl!: ReadableStreamDefaultController<ReadableStream<Uint8Array>>;
  private uniStreamCtl: ReadableStreamDefaultController<Uint8Array> | undefined;

  constructor(
    readonly url: string,
    readonly options?: { serverCertificateHashes?: { algorithm: string; value: Uint8Array }[] },
  ) {
    if (FakeWebTransport.mode === 'throw') throw new TypeError('FakeWebTransport: constructor rejects the options');
    FakeWebTransport.instances.push(this);
    this.closed = new Promise((r) => (this.resolveClosed = r));
    const mode = FakeWebTransport.mode;
    this.ready =
      mode === 'never-ready'
        ? new Promise(() => undefined)
        : mode === 'reject-ready'
          ? Promise.reject(new Error('FakeWebTransport: connection refused'))
          : Promise.resolve();
    this.ready.catch(() => undefined);
    const received: Uint8Array[] = [];
    this.conn = {
      receivedC0: [],
      receivedC1: received,
      c0: (b) => this.bidiCtl.enqueue(encodeStreamFrame(b)),
      c2: (b) => this.dgCtl.enqueue(encodeDatagram(Channel.C2, b)),
      c4: (b) => {
        if (!this.uniStreamCtl) {
          this.uniCtl.enqueue(
            new ReadableStream<Uint8Array>({
              start: (c) => {
                this.uniStreamCtl = c;
              },
            }),
          );
        }
        this.uniStreamCtl?.enqueue(encodeStreamFrame(b));
      },
      drop: (reason) => {
        this.resolveClosed({ closeCode: 1, reason: reason ?? 'dropped' });
      },
    };
    const dgReadable = new ReadableStream<Uint8Array>({ start: (c) => (this.dgCtl = c) });
    this.incomingUnidirectionalStreams = new ReadableStream<ReadableStream<Uint8Array>>({ start: (c) => (this.uniCtl = c) });
    this.datagrams = {
      readable: dgReadable,
      writable: new WritableStream<Uint8Array>({
        write: (chunk) => {
          const d = decodeDatagram(chunk);
          if (d.channel === Channel.C1) received.push(d.payload.slice());
        },
      }),
      maxDatagramSize: FakeWebTransport.maxDatagram,
    };
  }

  createBidirectionalStream(): Promise<{ readable: ReadableStream<Uint8Array>; writable: WritableStream<Uint8Array> }> {
    const readable = new ReadableStream<Uint8Array>({ start: (c) => (this.bidiCtl = c) });
    const framer = new StreamFrameReader((payload) => {
      this.conn.receivedC0.push(payload);
      if (FakeWebTransport.mode === 'no-answer') return;
      const a = answerC0(FakeWebTransport.server, payload);
      if (a) setTimeout(() => this.bidiCtl.enqueue(encodeStreamFrame(a)), 0);
    });
    const writable = new WritableStream<Uint8Array>({ write: (chunk) => framer.push(chunk) });
    return Promise.resolve({ readable, writable });
  }

  close(): void {
    this.closedByClient = true;
    this.resolveClosed({ closeCode: 0, reason: 'client close' });
  }

  static reset(server: FakeServerOpts, mode: FakeWtMode = 'ok'): void {
    FakeWebTransport.server = server;
    FakeWebTransport.mode = mode;
    FakeWebTransport.instances = [];
    FakeWebTransport.maxDatagram = 1200;
  }
}
