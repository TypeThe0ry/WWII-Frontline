/**
 * WebTransport transport. Wire format (ARCHITECTURE §6.6.1, DEVELOPING.md):
 *   C0  the client's first bidirectional stream, `[len u16 LE][payload]` both ways
 *   C1  datagram up `[1][payload]`;  C2/C3 datagram down `[ch][payload]`
 *   C4  the server's first unidirectional stream, `[len u16 LE][payload]`
 * The server only learns about the bidi stream once the client writes to it (the Hello).
 */
import { Channel, decodeDatagram, encodeDatagram, encodeStreamFrame, StreamFrameReader } from './codec';
import { Handlers, type Transport, type TransportCounters, type Unsubscribe } from './transport';

// Structural subset of the WebTransport API (so tests can inject fakes and old DOM libs do not matter).
export interface WtStreamReader {
  read(): Promise<{ value?: Uint8Array; done: boolean }>;
  cancel(reason?: unknown): Promise<void>;
}
export interface WtStreamWriter {
  write(chunk: Uint8Array): Promise<void>;
  readonly ready: Promise<void>;
  readonly desiredSize: number | null;
}
export interface WtReadable<T = Uint8Array> {
  getReader(): { read(): Promise<{ value?: T; done: boolean }>; cancel(reason?: unknown): Promise<void> };
}
export interface WtWritable {
  getWriter(): WtStreamWriter;
}
export interface WebTransportLike {
  readonly ready: Promise<unknown>;
  readonly closed: Promise<unknown>;
  readonly datagrams: { readonly readable: WtReadable; readonly writable: WtWritable; readonly maxDatagramSize: number };
  readonly incomingUnidirectionalStreams: WtReadable<WtReadable>;
  createBidirectionalStream(): Promise<{ readable: WtReadable; writable: WtWritable }>;
  close(info?: { closeCode?: number; reason?: string }): void;
}
export type WebTransportCtor = new (
  url: string,
  options?: { serverCertificateHashes?: { algorithm: string; value: Uint8Array }[] },
) => WebTransportLike;

export class WtTransport implements Transport {
  readonly kind = 'wt' as const;
  readonly counters: TransportCounters = { bytesIn: 0, bytesOut: 0, pktsIn: 0, pktsOut: 0 };
  /** C1 datagrams dropped locally (larger than the datagram budget). */
  droppedC1 = 0;

  private wt: WebTransportLike | undefined;
  private c0Writer: WtStreamWriter | undefined;
  private dgWriter: WtStreamWriter | undefined;
  private maxDg = 1200;
  private ready = false;
  private closedLocally = false;
  private readonly h = {
    c0: new Handlers<[Uint8Array]>(),
    c2: new Handlers<[Uint8Array]>(),
    c3: new Handlers<[Uint8Array]>(),
    c4: new Handlers<[Uint8Array]>(),
    close: new Handlers<[string]>(),
  };

  constructor(
    private readonly url: string,
    private readonly certHash?: Uint8Array,
    private readonly Ctor: WebTransportCtor = (globalThis as { WebTransport?: WebTransportCtor }).WebTransport as WebTransportCtor,
  ) {}

  /** Largest C1 payload (datagram budget minus the channel byte). */
  get maxDatagramSize(): number {
    return Math.max(0, this.maxDg - 1);
  }

  async connect(): Promise<void> {
    // serverCertificateHashes only when a dev certificate hash was given (M0-017); otherwise the normal trust chain applies.
    const opts = this.certHash ? { serverCertificateHashes: [{ algorithm: 'sha-256', value: this.certHash }] } : undefined;
    const wt = opts ? new this.Ctor(this.url, opts) : new this.Ctor(this.url);
    this.wt = wt;
    // Avoid unhandled rejections for the promises we do not await on the failure path.
    wt.closed.then(
      (info) => this.onSessionClosed(info),
      (e) => this.onSessionClosed(e),
    );
    await wt.ready;
    this.ready = true;
    this.maxDg = wt.datagrams.maxDatagramSize;
    const bidi = await wt.createBidirectionalStream();
    this.c0Writer = bidi.writable.getWriter();
    this.dgWriter = wt.datagrams.writable.getWriter();
    void this.pumpStream(bidi.readable.getReader(), this.h.c0);
    void this.pumpDatagrams(wt.datagrams.readable.getReader());
    void this.pumpUni(wt.incomingUnidirectionalStreams.getReader());
  }

  private onSessionClosed(info: unknown): void {
    if (this.closedLocally || !this.ready) return;
    this.ready = false;
    const r = info as { reason?: string; closeCode?: number; message?: string } | undefined;
    const why = r?.reason || r?.message || (r?.closeCode !== undefined ? `code ${r.closeCode}` : 'closed');
    this.h.close.emit(`webtransport closed: ${why}`);
  }

  private async pumpStream(reader: ReturnType<WtReadable['getReader']>, sink: Handlers<[Uint8Array]>): Promise<void> {
    const framer = new StreamFrameReader((payload) => sink.emit(payload));
    try {
      for (;;) {
        const { value, done } = await reader.read();
        if (done) break;
        if (value) {
          this.counters.bytesIn += value.length;
          this.counters.pktsIn++;
          framer.push(value);
        }
      }
    } catch {
      /* session closed: reported through wt.closed */
    }
  }

  private async pumpUni(reader: ReturnType<WtReadable<WtReadable>['getReader']>): Promise<void> {
    try {
      for (;;) {
        const { value, done } = await reader.read();
        if (done || !value) break;
        // Only the server's first uni stream is C4; further streams are tolerated and read the same way.
        void this.pumpStream(value.getReader(), this.h.c4);
      }
    } catch {
      /* closed */
    }
  }

  private async pumpDatagrams(reader: ReturnType<WtReadable['getReader']>): Promise<void> {
    try {
      for (;;) {
        const { value, done } = await reader.read();
        if (done) break;
        if (!value) continue;
        this.counters.bytesIn += value.length;
        this.counters.pktsIn++;
        let d: { channel: Channel; payload: Uint8Array };
        try {
          d = decodeDatagram(value);
        } catch {
          continue;
        }
        if (d.channel === Channel.C2) this.h.c2.emit(d.payload);
        else if (d.channel === Channel.C3) this.h.c3.emit(d.payload);
      }
    } catch {
      /* closed */
    }
  }

  sendC0(bytes: Uint8Array): void {
    const w = this.c0Writer;
    if (!w) return;
    const frame = encodeStreamFrame(bytes);
    this.counters.bytesOut += frame.length;
    this.counters.pktsOut++;
    w.write(frame).catch(() => undefined);
  }

  sendC1(bytes: Uint8Array): void {
    const w = this.dgWriter;
    if (!w) return;
    if (bytes.length > this.maxDatagramSize) {
      this.droppedC1++;
      return;
    }
    const dg = encodeDatagram(Channel.C1, bytes);
    this.counters.bytesOut += dg.length;
    this.counters.pktsOut++;
    // Datagrams are unreliable and the browser queue sheds the oldest itself; never await here.
    w.write(dg).catch(() => undefined);
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
    try {
      this.wt?.close({ closeCode: 0, reason: 'client close' });
    } catch {
      /* already closed */
    }
  }
}
