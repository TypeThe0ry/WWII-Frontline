/** Hello -> Welcome | Reject handshake on C0 (ARCHITECTURE §6.8.1). */
import type { NetCodec, WelcomeMsg } from './codec';
import type { Transport } from './transport';

export type NetErrorCode =
  | 'rejected' //        server answered Reject (version / data hash / full / banned / ticket)
  | 'data_hash_mismatch' // Welcome.data_hash differs from the local data
  | 'handshake_timeout'
  | 'transport_failed' // could not establish the transport
  | 'closed'; //          transport went away during the handshake

/** Error with a machine-readable code and a message fit for display. */
export class NetError extends Error {
  constructor(
    readonly code: NetErrorCode,
    message: string,
    readonly reason?: string,
  ) {
    super(message);
    this.name = 'NetError';
  }
}

export const HANDSHAKE_TIMEOUT_MS = 5000;

export interface HandshakeOptions {
  protocolVersion?: number;
  dataHash?: bigint;
  clientBuild?: number;
  caps?: number;
  ticket?: Uint8Array;
  timeoutMs?: number;
  now?: () => number;
}

export interface HandshakeResult {
  welcome: WelcomeMsg;
  /** Hello sent -> Welcome received, ms. */
  rttMs: number;
  recvAt: number;
}

const REJECT_TEXT: Record<string, string> = {
  version_mismatch: 'client and server protocol versions differ - reload the page to get the current client',
  data_hash_mismatch: 'client and server game data differ - reload the page to get the current client',
  server_full: 'the server is full',
  banned: 'you are banned from this server',
  invalid_ticket: 'the join ticket is invalid',
};

export function rejectMessage(reasonName: string, protocolVersion: number): string {
  return `Server rejected the connection: ${reasonName} (${REJECT_TEXT[reasonName] ?? 'unknown reason'}; client protocol v${protocolVersion})`;
}

export function handshake(t: Transport, codec: NetCodec, o: HandshakeOptions = {}): Promise<HandshakeResult> {
  const now = o.now ?? (() => performance.now());
  const protocolVersion = o.protocolVersion ?? codec.protocolVersion;
  const localHash = o.dataHash ?? codec.dataHash;
  return new Promise<HandshakeResult>((resolve, reject) => {
    let sentAt = 0;
    const done = (fn: () => void): void => {
      clearTimeout(timer);
      offC0();
      offClose();
      fn();
    };
    const timer = setTimeout(
      () => done(() => reject(new NetError('handshake_timeout', `No answer from the server within ${(o.timeoutMs ?? HANDSHAKE_TIMEOUT_MS) / 1000} s`))),
      o.timeoutMs ?? HANDSHAKE_TIMEOUT_MS,
    );
    const offClose = t.onClose((why) => done(() => reject(new NetError('closed', `Connection lost during handshake: ${why}`))));
    const offC0 = t.onC0((bytes) => {
      const recvAt = now();
      let msg;
      try {
        msg = codec.decodeServerC0(bytes);
      } catch (e) {
        done(() => reject(new NetError('transport_failed', `Malformed handshake answer: ${e instanceof Error ? e.message : String(e)}`)));
        return;
      }
      if (msg.kind === 'welcome') {
        if (msg.dataHash !== localHash) {
          done(() =>
            reject(
              new NetError(
                'data_hash_mismatch',
                `Game data mismatch: server ${msg.dataHash.toString(16)}, client ${localHash.toString(16)}`,
                'data_hash_mismatch',
              ),
            ),
          );
          return;
        }
        done(() => resolve({ welcome: msg, rttMs: recvAt - sentAt, recvAt }));
      } else if (msg.kind === 'reject') {
        done(() => reject(new NetError('rejected', rejectMessage(msg.reasonName, protocolVersion), msg.reasonName)));
      } else if (msg.kind === 'disconnect') {
        done(() => reject(new NetError('closed', `Server closed the connection: ${msg.reasonName}`, msg.reasonName)));
      }
      // A Pong cannot precede the Welcome; ignore anything else.
    });
    const hello = codec.encodeHello({
      protocolVersion,
      dataHash: localHash,
      clientBuild: o.clientBuild ?? 0,
      caps: o.caps ?? 0,
      ...(o.ticket ? { ticket: o.ticket } : {}),
    });
    sentAt = now();
    t.sendC0(hello);
  });
}
