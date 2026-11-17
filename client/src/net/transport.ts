/** Transport abstraction (ARCHITECTURE §6.6.1): WebTransport preferred, WebSocket fallback. */

export type TransportKind = 'wt' | 'ws';

export type Unsubscribe = () => void;

export interface TransportCounters {
  bytesIn: number;
  bytesOut: number;
  pktsIn: number;
  pktsOut: number;
}

export interface Transport {
  readonly kind: TransportKind;
  /** Largest C1 payload that fits one packet (WT: datagram budget; WS: 65535). Valid after `connect()`. */
  readonly maxDatagramSize: number;
  readonly counters: TransportCounters;
  /** Resolves when the transport is usable; rejects when it cannot be established. */
  connect(): Promise<void>;
  /** Reliable, ordered (C0). */
  sendC0(bytes: Uint8Array): void;
  /** Unreliable, latest wins (C1). */
  sendC1(bytes: Uint8Array): void;
  onC0(cb: (bytes: Uint8Array) => void): Unsubscribe;
  onC2(cb: (bytes: Uint8Array) => void): Unsubscribe;
  onC3(cb: (bytes: Uint8Array) => void): Unsubscribe;
  onC4(cb: (bytes: Uint8Array) => void): Unsubscribe;
  /** Fires once when an established transport goes away (not for `close()` called locally). */
  onClose(cb: (reason: string) => void): Unsubscribe;
  close(): void;
}

/** Small multicast helper shared by both transports. */
export class Handlers<A extends unknown[]> {
  private list: ((...a: A) => void)[] = [];

  add(cb: (...a: A) => void): Unsubscribe {
    this.list = [...this.list, cb];
    return () => {
      this.list = this.list.filter((f) => f !== cb);
    };
  }

  emit(...a: A): void {
    const l = this.list;
    for (let i = 0; i < l.length; i++) {
      try {
        (l[i] as (...a: A) => void)(...a);
      } catch (e) {
        console.error('[net] handler threw', e);
      }
    }
  }

  clear(): void {
    this.list = [];
  }
}
