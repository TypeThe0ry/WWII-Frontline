/** WebSocket transport: every channel multiplexed on one connection as `[channel u8][len u16][payload]`. */
import { Channel, decodeWsFrame, encodeWsFrame } from './codec';
import { Handlers, type Transport, type TransportCounters, type Unsubscribe } from './transport';

/** C1 backlog limit: above this many buffered bytes only the newest input packet is kept (latest wins). */
export const WS_C1_BUFFER_LIMIT = 8 * 1024;
const FLUSH_POLL_MS = 4;

export interface WebSocketLike {
  binaryType: string;
  readonly readyState: number;
  readonly bufferedAmount: number;
  onopen: ((ev: unknown) => void) | null;
  onmessage: ((ev: { data: unknown }) => void) | null;
  onerror: ((ev: unknown) => void) | null;
  onclose: ((ev: { code?: number; reason?: string }) => void) | null;
  send(data: Uint8Array): void;
  close(code?: number, reason?: string): void;
}
export type WebSocketCtor = new (url: string) => WebSocketLike;

const OPEN = 1;

export class WsTransport implements Transport {
  readonly kind = 'ws' as const;
  readonly maxDatagramSize = 65535;
  readonly counters: TransportCounters = { bytesIn: 0, bytesOut: 0, pktsIn: 0, pktsOut: 0 };
  /** C1 packets replaced by newer ones while the socket was congested. */
  droppedC1 = 0;

  private ws: WebSocketLike | undefined;
  private established = false;
  private closedLocally = false;
  private pendingC1: Uint8Array | undefined;
  private flushTimer: ReturnType<typeof setTimeout> | undefined;
  private readonly h = {
    c0: new Handlers<[Uint8Array]>(),
    c2: new Handlers<[Uint8Array]>(),
    c3: new Handlers<[Uint8Array]>(),
    c4: new Handlers<[Uint8Array]>(),
    close: new Handlers<[string]>(),
  };

  constructor(
    private readonly url: string,
    private readonly Ctor: WebSocketCtor = globalThis.WebSocket as unknown as WebSocketCtor,
  ) {}

  connect(): Promise<void> {
    return new Promise<void>((resolve, reject) => {
      let ws: WebSocketLike;
      try {
        ws = new this.Ctor(this.url);
      } catch (e) {
        reject(e instanceof Error ? e : new Error(String(e)));
        return;
      }
      this.ws = ws;
      ws.binaryType = 'arraybuffer';
      ws.onopen = () => {
        this.established = true;
        resolve();
      };
      ws.onerror = () => {
        if (!this.established) reject(new Error(`WebSocket connection to ${this.url} failed`));
      };
      ws.onmessage = (ev) => this.onMessage(ev.data);
      ws.onclose = (ev) => {
        this.stopFlush();
        if (!this.established) {
          reject(new Error(`WebSocket closed before open (code ${ev.code ?? 0})`));
          return;
        }
        this.established = false;
        if (!this.closedLocally) this.h.close.emit(ev.reason ? `ws closed: ${ev.reason}` : `ws closed (code ${ev.code ?? 0})`);
      };
    });
  }

  private onMessage(data: unknown): void {
    let buf: Uint8Array;
    if (data instanceof ArrayBuffer) buf = new Uint8Array(data);
    else if (ArrayBuffer.isView(data)) buf = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
    else return; // text frames are not part of the protocol
    this.counters.bytesIn += buf.length;
    this.counters.pktsIn++;
    let f: { channel: Channel; payload: Uint8Array };
    try {
      f = decodeWsFrame(buf);
    } catch {
      return; // malformed frame: ignore (the server never sends one; do not let it throw into the socket)
    }
    switch (f.channel) {
      case Channel.C0:
        this.h.c0.emit(f.payload);
        break;
      case Channel.C2:
        this.h.c2.emit(f.payload);
        break;
      case Channel.C3:
        this.h.c3.emit(f.payload);
        break;
      case Channel.C4:
        this.h.c4.emit(f.payload);
        break;
      default:
        break;
    }
  }

  private write(frame: Uint8Array): void {
    const ws = this.ws;
    if (!ws || ws.readyState !== OPEN) return;
    ws.send(frame);
    this.counters.bytesOut += frame.length;
    this.counters.pktsOut++;
  }

  sendC0(bytes: Uint8Array): void {
    this.write(encodeWsFrame(Channel.C0, bytes));
  }

  sendC1(bytes: Uint8Array): void {
    const ws = this.ws;
    if (!ws || ws.readyState !== OPEN) return;
    const frame = encodeWsFrame(Channel.C1, bytes);
    if (ws.bufferedAmount > WS_C1_BUFFER_LIMIT) {
      // Congested: keep only the newest packet and send it once the backlog drains.
      if (this.pendingC1) this.droppedC1++;
      this.pendingC1 = frame;
      this.scheduleFlush();
      return;
    }
    if (this.pendingC1) {
      this.droppedC1++; // an older packet is superseded by this one
      this.pendingC1 = undefined;
    }
    this.write(frame);
  }

  private scheduleFlush(): void {
    if (this.flushTimer !== undefined) return;
    this.flushTimer = setTimeout(() => {
      this.flushTimer = undefined;
      const ws = this.ws;
      if (!ws || ws.readyState !== OPEN) {
        this.pendingC1 = undefined;
        return;
      }
      if (ws.bufferedAmount > WS_C1_BUFFER_LIMIT) {
        this.scheduleFlush();
        return;
      }
      const p = this.pendingC1;
      this.pendingC1 = undefined;
      if (p) this.write(p);
    }, FLUSH_POLL_MS);
  }

  private stopFlush(): void {
    if (this.flushTimer !== undefined) clearTimeout(this.flushTimer);
    this.flushTimer = undefined;
    this.pendingC1 = undefined;
  }

  onC0(cb: (b: Uint8Array) => void): Unsubscribe {
    return this.h.c0.add(cb);
  }
  onC2(cb: (b: Uint8Array) => void): Unsubscribe {
    return this.h.c2.add(cb);
  }
  onC3(cb: (b: Uint8Array) => void): Unsubscribe {
    return this.h.c3.add(cb);
  }
  onC4(cb: (b: Uint8Array) => void): Unsubscribe {
    return this.h.c4.add(cb);
  }
  onClose(cb: (reason: string) => void): Unsubscribe {
    return this.h.close.add(cb);
  }

  close(): void {
    this.closedLocally = true;
    this.stopFlush();
    const ws = this.ws;
    if (ws) {
      ws.onopen = ws.onmessage = ws.onerror = null;
      try {
        ws.close(1000, 'client close');
      } catch {
        /* already closed */
      }
    }
  }
}
