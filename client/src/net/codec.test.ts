import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeAll, describe, expect, it } from 'vitest';
import { Channel, decodeDatagram, decodeWsFrame, encodeDatagram, encodeStreamFrame, encodeWsFrame, FrameError, NetCodec, StreamFrameReader } from './codec';
import { HAVE_WASM, loadWasm, WASM_HOW } from './test-support';

const hex = (s: string): Uint8Array => Uint8Array.from(s.match(/../g) ?? [], (b) => parseInt(b, 16));
const GOLDEN_DIR = fileURLToPath(new URL('../../../crates/protocol/tests/golden/', import.meta.url));

describe('framing (golden bytes of crates/protocol/src/framing.rs)', () => {
  it('encodes exactly like protocol::framing', () => {
    expect([...encodeWsFrame(Channel.C0, Uint8Array.of(0xaa, 0xbb))]).toEqual([0x00, 0x02, 0x00, 0xaa, 0xbb]);
    expect([...encodeWsFrame(Channel.C2, new Uint8Array(0))]).toEqual([0x02, 0x00, 0x00]);
    const f = encodeWsFrame(Channel.C4, new Uint8Array(0x0102).fill(7));
    expect([...f.subarray(0, 3)]).toEqual([0x04, 0x02, 0x01]);
    expect(f.length).toBe(3 + 0x0102);
  });

  it('limits and errors match the Rust side', () => {
    const max = new Uint8Array(65535);
    expect(decodeWsFrame(encodeWsFrame(Channel.C3, max)).payload.length).toBe(65535);
    expect(() => encodeWsFrame(Channel.C3, new Uint8Array(65536))).toThrow(FrameError);
    expect(() => decodeWsFrame(new Uint8Array(0))).toThrow(/shorter/);
    expect(() => decodeWsFrame(Uint8Array.of(0, 0))).toThrow(/shorter/);
    expect(() => decodeWsFrame(Uint8Array.of(9, 0, 0))).toThrow(/channel/);
    expect(() => decodeWsFrame(Uint8Array.of(0, 1, 0))).toThrow(/length/);
    expect(() => decodeWsFrame(Uint8Array.of(0, 0, 0, 1))).toThrow(/length/);
  });

  it('round-trips every channel and rejects every strict prefix', () => {
    for (let ch = 0; ch <= 5; ch++) {
      const payload = Uint8Array.from({ length: 300 }, (_, i) => (i * 7 + ch) & 255);
      const f = encodeWsFrame(ch as Channel, payload);
      const d = decodeWsFrame(f);
      expect(d.channel).toBe(ch);
      expect([...d.payload]).toEqual([...payload]);
      for (let n = 0; n < f.length; n++) expect(() => decodeWsFrame(f.subarray(0, n))).toThrow(FrameError);
    }
  });

  it('stream frames [len u16 LE][payload] and datagrams [ch][payload]', () => {
    expect([...encodeStreamFrame(Uint8Array.of(1, 2, 3))]).toEqual([3, 0, 1, 2, 3]);
    expect([...encodeDatagram(Channel.C1, Uint8Array.of(9, 8))]).toEqual([1, 9, 8]);
    expect(decodeDatagram(Uint8Array.of(2, 5, 6))).toMatchObject({ channel: 2 });
    expect(() => decodeDatagram(new Uint8Array(0))).toThrow(FrameError);
    expect(() => decodeDatagram(Uint8Array.of(77, 1))).toThrow(FrameError);
  });

  it('StreamFrameReader reassembles frames split at any byte boundary', () => {
    const frames = [Uint8Array.of(1), new Uint8Array(0), Uint8Array.from({ length: 700 }, (_, i) => i & 255), Uint8Array.of(4, 4)];
    const wire = Uint8Array.from(frames.flatMap((f) => [...encodeStreamFrame(f)]));
    for (const chunk of [1, 2, 3, 5, 64, wire.length]) {
      const got: number[][] = [];
      const r = new StreamFrameReader((p) => got.push([...p]));
      for (let i = 0; i < wire.length; i += chunk) r.push(wire.subarray(i, i + chunk));
      expect(got).toEqual(frames.map((f) => [...f]));
      expect(r.pending).toBe(0);
    }
  });
});

describe.skipIf(!HAVE_WASM)('message codec over sim-wasm (golden messages of crates/protocol)', () => {
  let codec: NetCodec;
  beforeAll(async () => {
    codec = new NetCodec(await loadWasm());
  });

  it('decodes the protocol golden Welcome / Pong / Reject / Disconnect', () => {
    expect(codec.decodeServerC0(hex('020300015f901edeadbeef000000010123456789abcdef010001fffc'))).toEqual({
      kind: 'welcome',
      playerId: 3,
      serverTick: 90000,
      tickHz: 30,
      matchSeed: 0xdeadbeef00000001n,
      dataHash: 0x0123456789abcdefn,
      mapId: 1,
      mode: 0,
      team: 1,
      clientTickOffset: -4,
    });
    expect(codec.decodeServerC0(hex('05000004d200015f91002dc6e1'))).toEqual({ kind: 'pong', clientTime: 1234, serverTick: 90001, serverTimeMs: 3000033 });
    expect(codec.decodeServerC0(hex('0305'))).toEqual({ kind: 'reject', reason: 5, reasonName: 'data_hash_mismatch' });
    expect(codec.decodeServerC0(hex('0600'))).toEqual({ kind: 'disconnect', reason: 0, reasonName: 'normal' });
    expect(() => codec.decodeServerC0(hex('0700'))).toThrow();
    expect(() => codec.decodeServerC0(new Uint8Array(0))).toThrow();
  });

  it('encodes Ping like the protocol golden and Hello with the wasm data hash', () => {
    expect([...codec.encodePing(1234)]).toEqual([...hex('04000004d2')]);
    const h = codec.encodeHello({ protocolVersion: 1, dataHash: codec.dataHash, clientBuild: 0, caps: 2 });
    expect(h[0]).toBe(1);
    expect(h.length).toBe(18);
  });

  it.skipIf(!existsSync(`${GOLDEN_DIR}welcome.hex`))('golden files on disk equal the inline copies', () => {
    const f = (n: string): string => readFileSync(`${GOLDEN_DIR}${n}.hex`, 'utf8').trim();
    expect(f('welcome')).toBe('020300015f901edeadbeef000000010123456789abcdef010001fffc');
    expect(f('pong')).toBe('05000004d200015f91002dc6e1');
    expect(f('ping')).toBe('04000004d2');
    expect(f('reject')).toBe('0305');
  });
});

if (!HAVE_WASM) console.warn(`[net] sim-wasm package missing, message codec tests skipped: ${WASM_HOW}`);
