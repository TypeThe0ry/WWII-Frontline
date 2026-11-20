import { describe, expect, it } from 'vitest';
import { InputSampler } from '../input/sampler';
import { LAYOUT_SELF_CORR_X, LAYOUT_SELF_CORR_Y, LAYOUT_SELF_CORR_Z, LAYOUT_SELF_HAS_CORR, LAYOUT_SELF_HEALTH, LAYOUT_SELF_LEN, LAYOUT_SELF_PRESENT, LAYOUT_SELF_VEHICLE_SLOT } from '../net/wasm';
import { EventBus, type GameEvents } from './event-bus';
import { GameLoop } from './game-loop';
import { FakeSnapshotDecoder, HAVE_WASM, loadWasm } from './test-harness';

const spawn = { x: 2020, y: -6, z: 1980, yaw: 0 };

async function setup({ terrain = true, fireOriginHints = true } = {}) {
  const wasm = await loadWasm();
  const input = new InputSampler();
  const packets: Uint8Array[] = [];
  let target = 101;
  let snapshotTick = 100;
  const game = new GameLoop({
    wasm, decoder: new FakeSnapshotDecoder(), bus: new EventBus<GameEvents>(),
    net: {
      clock: { inputTargetTick: () => target, estimatedServerTick: () => target - 2, now: () => target - 2, applySlack: () => undefined, rtt: () => 0 },
      sendInputBundle: (b) => packets.push(b),
    },
    input, matchSeed: 1n, mapId: 1, playerId: 3, spawn, fireOriginHints,
  });
  if (terrain) {
    const flat = new Uint8Array(16 + 65 * 65 * 2 + 64 * 64);
    flat.set([70, 67, 79, 76, 1, 0, 65, 0]); // FCOL, v1, 65 samples
    const header = new DataView(flat.buffer);
    header.setFloat32(8, -7, true);
    header.setFloat32(12, -7, true);
    game.loadCollision([{ bx: 31, bz: 30, data: flat }]);
  }
  const step = (): Uint8Array => {
    game.update(1, 1, 1000 / 30);
    target++;
    return packets.at(-1)!;
  };
  const snapshot = (ack: number, { x = spawn.x, vehicle = 0, health = 100 } = {}): void => {
    const self = new Float32Array(LAYOUT_SELF_LEN);
    self[LAYOUT_SELF_PRESENT] = 1;
    self[LAYOUT_SELF_HAS_CORR] = 1;
    self[LAYOUT_SELF_CORR_X] = x;
    self[LAYOUT_SELF_CORR_Y] = spawn.y;
    self[LAYOUT_SELF_CORR_Z] = spawn.z;
    self[LAYOUT_SELF_HEALTH] = health;
    self[LAYOUT_SELF_VEHICLE_SLOT] = vehicle;
    game.onSnapshot(FakeSnapshotDecoder.wire({
      tick: snapshotTick++, ackInputSeq: ack, inputSlack: 0, self,
      ids: [], pos: [], yaw: [], pitch: [], removed: [], events: new Uint8Array(),
    }));
  };
  return { game, input, packets, step, snapshot, wasm };
}

// MSB-first C1 header: ack(16), pred_hash(8), input count(2), fire count(2).
const fireCount = (packet: Uint8Array): number => (packet[3]! >> 4) & 3;
const readBits = (packet: Uint8Array, offset: number, width: number): number => {
  let value = 0;
  for (let i = offset; i < offset + width; i++) value = (value << 1) | ((packet[i >> 3]! >> (7 - (i & 7))) & 1);
  return value;
};

describe.skipIf(!HAVE_WASM)('production C1 fire origin wiring', () => {
  it('attaches a nonzero hint to the shot input, repeats it unchanged, and drops it on ACK', async () => {
    const { game, input, packets, step, snapshot, wasm } = await setup();
    try {
      snapshot(0); // Initial authoritative pose can be within reconciliation tolerance.
      input.press('Mouse0');
      const first = step();
      const shotEntry = game.ring.at(0);
      expect(fireCount(first)).toBe(1);
      const fireOffset = 28 + 144; // Header, then one full-width InputCmd.
      expect(readBits(first, fireOffset, 16)).toBe(shotEntry.seq);
      expect(readBits(first, fireOffset + 16, 16)).toBe(5);
      expect(readBits(first, fireOffset + 32, 8)).toBe(readBits(first, 28 + 112, 8)); // InputCmd.weapon_slot
      const originOffset = fireOffset + 16 + 16 + 8 + 16 + 16;
      const code = [0, 1, 2].map((i) => readBits(first, originOffset + 14 * i, 14));
      expect(code[0]! / 4).toBeCloseTo(spawn.x, 0);
      expect(code[1]! / 4 - 128).toBeCloseTo(-4.5, 0);
      expect(code[2]! / 4).toBe(spawn.z + 0.5);

      const originalFire = shotEntry.fire!.slice();
      snapshot(0, { x: spawn.x + 1 }); // Replays the pending shot from a different pose.
      expect(game.predictor.replaySteps).toBe(1);
      expect(packets).toHaveLength(1); // Replay itself never sends.
      const next = step(); // Holding a semi-auto trigger does not predict a second shot.
      expect(game.stats.fireLocal).toBe(1);
      const cmds = new Uint8Array(2 * shotEntry.cmd.length);
      cmds.set(shotEntry.cmd);
      cmds.set(game.ring.at(1).cmd, shotEntry.cmd.length);
      expect(next).toEqual(wasm.encodeInputBundle(cmds, 101, wasm.predHash(spawn.x + 1, spawn.y, spawn.z), originalFire));

      snapshot(1);
      expect(fireCount(step())).toBe(0);
    } finally {
      game.dispose();
    }
  });

  it.each(['before-position', 'map-mismatch', 'missing-terrain', 'vehicle', 'dead'] as const)('omits hints for %s without suppressing InputCmd', async (reason) => {
    const { game, input, step, snapshot } = await setup({ terrain: reason !== 'missing-terrain', fireOriginHints: reason !== 'map-mismatch' });
    try {
      if (reason !== 'before-position') snapshot(0, { vehicle: reason === 'vehicle' ? 128 : 0, health: reason === 'dead' ? 0 : 100 });
      input.press('Mouse0');
      const packet = step();
      expect(readBits(packet, 28 + 16 + 32 + 8 + 8 + 16 + 16, 16) & 1).toBe(1); // FIRE remains in InputCmd.
      expect(fireCount(packet)).toBe(0);
    } finally {
      game.dispose();
    }
  });
});
