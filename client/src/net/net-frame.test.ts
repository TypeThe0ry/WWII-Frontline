import { beforeAll, describe, expect, it } from 'vitest';
import { ClockSync } from './clock';
import { Channel, encodeWsFrame } from './codec';
import { FakeWebSocket, type FakeServerOpts } from './fakes';
import { HAVE_WASM, loadWasm } from './test-support';
import { WsTransport } from './ws-transport';

// Per-frame cost of the net module (budget 0.2 ms per frame, part of ARCHITECTURE §13.5.1): one 400-byte snapshot packet
// received and handed to the game (callback only enqueues, decoding is the game's job), the clock queried for the
// current tick and input lead, slack feedback applied, one input bundle sent, stats read.
describe.skipIf(!HAVE_WASM)('net per-frame cost', () => {
  let server: FakeServerOpts;
  beforeAll(async () => {
    const wasm = await loadWasm();
    server = { dataHash: wasm.dataHash(), protocolVersion: wasm.protocolVersion() };
  });

  it('WebSocket receive + clock + send stays below 0.2 ms per frame', async () => {
    FakeWebSocket.reset(server);
    const t = new WsTransport('ws://x/', FakeWebSocket);
    await t.connect();
    const ws = FakeWebSocket.instances[0] as FakeWebSocket;
    const queue: Uint8Array[] = [];
    t.onC2((b) => {
      if (queue.length > 8) queue.length = 0;
      queue.push(b);
    });
    let now = 0;
    const clock = new ClockSync({ now: () => now });
    clock.seed(1000, 40, 0);
    const wire = encodeWsFrame(Channel.C2, new Uint8Array(400).fill(3)).buffer as ArrayBuffer;
    const input = new Uint8Array(40);
    const frame = (): void => {
      now += 16.7;
      ws.onmessage?.({ data: wire });
      queue.length = 0;
      clock.applySlack(1, now);
      void clock.inputTargetTick(now);
      void clock.jitter();
      t.sendC1(input);
      void t.counters.bytesIn;
    };
    for (let i = 0; i < 20000; i++) frame(); // warm up the JIT
    const N = 200_000;
    const t0 = performance.now();
    for (let i = 0; i < N; i++) frame();
    const perFrameMs = (performance.now() - t0) / N;
    console.info(`net per-frame cost (WS, 1 x 400 B snapshot in, 1 input out, clock queries): ${(perFrameMs * 1000).toFixed(2)} us`);
    expect(perFrameMs).toBeLessThan(0.2);
    t.close();
  });
});
