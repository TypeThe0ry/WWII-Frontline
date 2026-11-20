/**
 * Framing and message codec glue (ARCHITECTURE §6.6, crates/protocol/src/framing.rs).
 *
 * - WebSocket: one frame per binary message, `[channel u8][len u16 LE][payload]`.
 * - WebTransport streams (C0 up/down, C4 down): `[len u16 LE][payload]`, the stream is the channel.
 * - WebTransport datagrams (C1 up, C2/C3 down): `[channel u8][payload]`.
 *
 * Message bodies are encoded/decoded by sim-wasm (`NetCodec`); nothing here touches bits.
 * `codec.test.ts` pins the byte layout against the golden bytes of `protocol::framing`.
 */
import type { C4Event, VoteKind } from '../game/state/c4';
import { LAYOUT_C0_DEPLOY_RESULT, LAYOUT_C0_DISCONNECT, LAYOUT_C0_PONG, LAYOUT_C0_REJECT, LAYOUT_C0_WELCOME, type SimWasmModule } from './wasm';

import { C0_CHAT, C0_NOTICE, type ChatChannel, type ChatMessage, type LegacyTeamMarker, type PingKind, type TeamMarkersFrame } from './comms';

export const Channel = { C0: 0, C1: 1, C2: 2, C3: 3, C4: 4, C5: 5 } as const;
export type Channel = (typeof Channel)[keyof typeof Channel];

export const FRAME_HEADER_LEN = 3;
export const MAX_FRAME_PAYLOAD = 0xffff;

export class FrameError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'FrameError';
  }
}

export function encodeWsFrame(channel: Channel, payload: Uint8Array): Uint8Array {
  if (payload.length > MAX_FRAME_PAYLOAD) throw new FrameError('payload exceeds 65535 bytes');
  const out = new Uint8Array(FRAME_HEADER_LEN + payload.length);
  out[0] = channel;
  out[1] = payload.length & 0xff;
  out[2] = payload.length >> 8;
  out.set(payload, FRAME_HEADER_LEN);
  return out;
}

/** Parses one WS message. Returns a view into `buf` (no copy); throws FrameError on malformed input. */
export function decodeWsFrame(buf: Uint8Array): { channel: Channel; payload: Uint8Array } {
  if (buf.length < FRAME_HEADER_LEN) throw new FrameError('frame shorter than header');
  const channel = buf[0] as number;
  if (channel > Channel.C5) throw new FrameError('unknown channel');
  const len = (buf[1] as number) | ((buf[2] as number) << 8);
  if (buf.length - FRAME_HEADER_LEN !== len) throw new FrameError('frame length field does not match payload');
  return { channel: channel as Channel, payload: buf.subarray(FRAME_HEADER_LEN) };
}

/** `[len u16 LE][payload]` for WebTransport streams. */
export function encodeStreamFrame(payload: Uint8Array): Uint8Array {
  if (payload.length > MAX_FRAME_PAYLOAD) throw new FrameError('payload exceeds 65535 bytes');
  const out = new Uint8Array(2 + payload.length);
  out[0] = payload.length & 0xff;
  out[1] = payload.length >> 8;
  out.set(payload, 2);
  return out;
}

/** Incremental parser for a byte stream of `[len u16 LE][payload]` frames (chunks may split frames anywhere). */
export class StreamFrameReader {
  private buf = new Uint8Array(0);

  constructor(private readonly onFrame: (payload: Uint8Array) => void) {}

  push(chunk: Uint8Array): void {
    let data = chunk;
    if (this.buf.length > 0) {
      const merged = new Uint8Array(this.buf.length + chunk.length);
      merged.set(this.buf);
      merged.set(chunk, this.buf.length);
      data = merged;
    }
    let off = 0;
    while (data.length - off >= 2) {
      const len = (data[off] as number) | ((data[off + 1] as number) << 8);
      if (data.length - off - 2 < len) break;
      // Copy: stream chunks may be reused/detached by the platform after the callback returns.
      this.onFrame(data.slice(off + 2, off + 2 + len));
      off += 2 + len;
    }
    this.buf = off === data.length ? new Uint8Array(0) : data.slice(off);
  }

  /** Bytes of an incomplete trailing frame. */
  get pending(): number {
    return this.buf.length;
  }
}

/** `[channel u8][payload]` for WebTransport datagrams. */
export function encodeDatagram(channel: Channel, payload: Uint8Array): Uint8Array {
  const out = new Uint8Array(1 + payload.length);
  out[0] = channel;
  out.set(payload, 1);
  return out;
}

export function decodeDatagram(buf: Uint8Array): { channel: Channel; payload: Uint8Array } {
  if (buf.length < 1) throw new FrameError('empty datagram');
  const channel = buf[0] as number;
  if (channel > Channel.C5) throw new FrameError('unknown channel');
  return { channel: channel as Channel, payload: buf.subarray(1) };
}

// --- messages -------------------------------------------------------------------------------

export const REJECT_REASONS: Readonly<Record<number, string>> = {
  1: 'version_mismatch',
  2: 'server_full',
  3: 'banned',
  4: 'invalid_ticket',
  5: 'data_hash_mismatch',
};

export const DISCONNECT_REASONS: Readonly<Record<number, string>> = {
  0: 'normal',
  1: 'timeout',
  2: 'kicked',
  3: 'server_shutdown',
  4: 'protocol_error',
  5: 'server_rule',
};

export interface WelcomeMsg {
  kind: 'welcome';
  playerId: number;
  serverTick: number;
  tickHz: number;
  matchSeed: bigint;
  dataHash: bigint;
  mapId: number;
  mode: number;
  team: number;
  clientTickOffset: number;
}
export interface RejectMsg {
  kind: 'reject';
  reason: number;
  reasonName: string;
}
export interface PongMsg {
  kind: 'pong';
  clientTime: number;
  serverTick: number;
  serverTimeMs: number;
}
export interface DisconnectMsg {
  kind: 'disconnect';
  reason: number;
  reasonName: string;
}
export interface DeployResultMsg {
  kind: 'deployResult';
  ok: boolean;
  spawnTick: number;
  reason: number;
}
export interface NoticeMsg {
  kind: 'notice';
  key: string;
}
export interface ChatMsg extends ChatMessage {
  kind: 'chat';
}
export type ServerMsg = WelcomeMsg | RejectMsg | PongMsg | DisconnectMsg | DeployResultMsg | NoticeMsg | ChatMsg;

export interface HelloParams {
  protocolVersion: number;
  dataHash: bigint;
  clientBuild: number;
  caps: number;
  ticket?: Uint8Array;
}

/** Thin typed wrapper over the sim-wasm message functions. */
export class NetCodec {
  constructor(readonly wasm: SimWasmModule) {}

  get protocolVersion(): number {
    return this.wasm.protocolVersion();
  }

  get dataHash(): bigint {
    return this.wasm.dataHash();
  }

  encodeHello(p: HelloParams): Uint8Array {
    return this.wasm.encodeHello(p.protocolVersion, p.dataHash, p.ticket ?? new Uint8Array(0), p.clientBuild, p.caps);
  }

  encodePing(clientTime: number): Uint8Array {
    return this.wasm.encodePing(clientTime);
  }

  encodeChat(channel: ChatChannel, text: string): Uint8Array {
    if (!Number.isInteger(channel) || channel < 0 || channel > 3) {
      throw new FrameError('invalid chat channel');
    }
    return this.wasm.encodeChat(channel, text);
  }

  /** X/Z are the same map-normalized u16 codes as C0 MarkerPlace. Target is an advisory hint. */
  encodePingPlace(kind: PingKind, pos: readonly [number, number], target = 0): Uint8Array {
    if (!Number.isInteger(kind) || kind < 0 || kind > 7 ||
        [...pos, target].some((v) => !Number.isInteger(v) || v < 0 || v > 0xffff)) {
      throw new FrameError('invalid PingPlace fields');
    }
    return this.wasm.encodePingPlace(kind, pos[0], pos[1], target);
  }

  decodeTeamMarkers(bytes: Uint8Array): TeamMarkersFrame {
    const data = this.wasm.decodeTeamMarkers(bytes);
    const markers: LegacyTeamMarker[] = [];
    for (let i = 1; i < data.length; i += 4) {
      markers.push({ kind: data[i]!, pos: [data[i + 1]!, data[i + 2]!], ttl: data[i + 3]! });
    }
    return { tick: data[0]!, markers };
  }

  encodeDisconnect(reason: number): Uint8Array {
    return this.wasm.encodeDisconnect(reason);
  }

  encodeVoteStart(kind: VoteKind, target: number): Uint8Array {
    return this.wasm.encodeVoteStart(kind, target);
  }

  encodeVoteCast(voteId: number, yes: boolean): Uint8Array {
    return this.wasm.encodeVoteCast(voteId, yes);
  }

  encodeTeamSwitchRequest(team: number, acceptOffer: boolean): Uint8Array {
    return this.wasm.encodeTeamSwitchRequest(team, acceptOffer);
  }

  encodeForgivePunish(victimEvent: number, punish: boolean): Uint8Array {
    return this.wasm.encodeForgivePunish(victimEvent, punish);
  }

  encodeMapVoteCast(voteId: number, choice: number): Uint8Array {
    return this.wasm.encodeMapVoteCast(voteId, choice);
  }

  private rosterSeq = 0;

  /**
   * Decodes one C4 payload (M1-040c). Roster entries get a receipt sequence number so stale incremental entries can be
   * told from fresh ones (`Roster` keeps the highest `seq`). Throws on malformed bytes or unknown event types.
   */
  decodeC4(bytes: Uint8Array): C4Event {
    const e = JSON.parse(this.wasm.decodeC4(bytes)) as C4Event;
    if (e.type === 'roster') return { ...e, seq: ++this.rosterSeq };
    return e;
  }

  /** Decodes a server C0 payload; throws on malformed bytes. The wasm object is freed before returning. */
  decodeServerC0(bytes: Uint8Array): ServerMsg {
    const m = this.wasm.decodeServerC0(bytes);
    try {
      switch (m.kind) {
        case LAYOUT_C0_WELCOME:
          return {
            kind: 'welcome',
            playerId: m.playerId,
            serverTick: m.serverTick,
            tickHz: m.tickHz,
            matchSeed: m.matchSeed,
            dataHash: m.dataHash,
            mapId: m.mapId,
            mode: m.mode,
            team: m.team,
            clientTickOffset: m.clientTickOffset,
          };
        case LAYOUT_C0_REJECT:
          return { kind: 'reject', reason: m.reason, reasonName: REJECT_REASONS[m.reason] ?? `code_${m.reason}` };
        case LAYOUT_C0_PONG:
          return { kind: 'pong', clientTime: m.clientTime, serverTick: m.serverTick, serverTimeMs: m.serverTimeMs };
        case LAYOUT_C0_DISCONNECT:
          return { kind: 'disconnect', reason: m.reason, reasonName: DISCONNECT_REASONS[m.reason] ?? `code_${m.reason}` };
        case LAYOUT_C0_DEPLOY_RESULT:
          return { kind: 'deployResult', ok: m.ok, spawnTick: m.spawnTick, reason: m.reason };
        case C0_CHAT:
          return { kind: 'chat', channel: m.chatChannel, text: m.chatText, sender: m.chatSender };
        case C0_NOTICE:
          return { kind: 'notice', key: m.noticeKey };
        default:
          throw new FrameError(`unexpected server C0 message kind ${m.kind}`);
      }
    } finally {
      m.free();
    }
  }
}
