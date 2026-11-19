import { describe, expect, it, vi } from 'vitest';
import { LAYOUT_SELF_HEALTH, LAYOUT_SELF_LEN, LAYOUT_SELF_PRESENT, LAYOUT_SELF_SEAT, LAYOUT_SELF_VEHICLE_SLOT } from '../net/wasm';
import { EventBus, type GameEvents } from './event-bus';
import { GameLoop } from './game-loop';
import { FakeSnapshotDecoder, HAVE_WASM, loadWasm, ScriptInput } from './test-harness';

async function setup() {
  let target = 11;
  const game = new GameLoop({
    wasm: await loadWasm(), decoder: new FakeSnapshotDecoder(), bus: new EventBus<GameEvents>(),
    net: {
      clock: { inputTargetTick: () => target, estimatedServerTick: () => target - 2, now: () => target - 2, applySlack: () => undefined, rtt: () => 0 },
      sendInputBundle: () => undefined,
    },
    input: new ScriptInput(() => ({ moveZ: 127 })), matchSeed: 1n, mapId: 0, playerId: 1,
  });
  const snap = (tick: number, slot: number, seat: number, rows: [id: number, x: number][] = [], ack = 0): void => {
    const self = new Float32Array(LAYOUT_SELF_LEN);
    self[LAYOUT_SELF_PRESENT] = 1;
    self[LAYOUT_SELF_HEALTH] = 100;
    self[LAYOUT_SELF_VEHICLE_SLOT] = slot;
    self[LAYOUT_SELF_SEAT] = seat;
    game.onSnapshot(FakeSnapshotDecoder.wire({
      tick, ackInputSeq: ack, inputSlack: 0, self, ids: [], pos: [], yaw: [], pitch: [], removed: [], events: new Uint8Array(0),
      vehIds: rows.map(([id]) => id),
      // Deliberately nonzero turret angles. Neither is the hull's yaw rate.
      vehState: rows.flatMap(([, x]) => [x, 0, 60, 0, 0, 0, 0, 0, 0, 1, 2, 0.1]),
      vehMeta: rows.flatMap(() => [1, 0, 255, 0]),
    }));
  };
  const step = (): void => {
    game.update(1, 1, 1000 / 30);
    target++;
  };
  return { game, snap, step };
}

describe.skipIf(!HAVE_WASM)('GameLoop vehicle seats', () => {
  it('waits for the selected hull row and switches vehicles, seats and back to foot', async () => {
    const { game, snap, step } = await setup();
    try {
      snap(7, 0, 0, [[129, 70]]);
      snap(8, 129, 0, [[128, 100]]);
      expect(game.vehiclePredictor).toBeUndefined();
      snap(10, 129, 0, [[128, 101], [129, 72]]);
      const v = game.vehiclePredictor!;
      expect(v.state[0]).toBe(72);
      expect(v.state[10]).toBe(0);

      const driverStep = vi.spyOn(v, 'step');
      const infantryTick = game.predictor.tick;
      step();
      expect(driverStep).toHaveBeenCalledExactlyOnceWith(11, game.ring.at(0).cmd);
      expect(game.predictor.tick).toBe(infantryTick + 1);
      driverStep.mockClear();

      snap(11, 129, 1, [[129, 73]], 1);
      expect(game.vehiclePredictor).toBeUndefined();
      step();
      expect(driverStep).not.toHaveBeenCalled();
      snap(10, 129, 0, [[129, 99]]); // a stale self block cannot restore driving
      expect(game.vehiclePredictor).toBeUndefined();

      snap(12, 129, 0, [[129, 74]], 2);
      expect(game.vehiclePredictor?.state[0]).toBe(74);
      snap(13, 128, 0, [[129, 75], [128, 101]], 2);
      expect(game.vehiclePredictor?.state[0]).toBe(101);
      snap(14, 0, 0, [[128, 102]], 2);
      expect(game.vehiclePredictor).toBeUndefined();
      expect(v.active).toBe(false);
      step();
      expect(driverStep).not.toHaveBeenCalled();
    } finally {
      game.dispose();
    }
  });

  it('keeps driving across snapshots which omit the seated hull row', async () => {
    const { game, snap, step } = await setup();
    try {
      snap(10, 129, 0, [[129, 72]]);
      const v = game.vehiclePredictor!;
      const correction = vi.spyOn(v, 'reconcile');
      step(); // seq 1 for tick 11
      step(); // seq 2 for tick 12
      const before = [...v.state];
      snap(11, 129, 0, [[128, 500]], 1);
      expect(correction).not.toHaveBeenCalled();
      expect(game.vehiclePredictor).toBe(v);
      expect([...v.state]).toEqual(before);
    } finally {
      game.dispose();
    }
  });

  it('ignores a small same-tick error and snaps a large one, replaying only future unacked commands', async () => {
    const { game, snap, step } = await setup();
    try {
      snap(10, 129, 0, [[129, 72]]);
      const v = game.vehiclePredictor!;
      const correction = vi.spyOn(v, 'reconcile');
      step(); // seq 1 for tick 11
      step(); // seq 2 for tick 12
      const before = [...v.state];
      snap(11, 129, 0, [[129, 72.01]], 1);
      expect(correction).toHaveReturnedWith('none');
      expect([...v.state]).toEqual(before);
      step(); // seq 3 for tick 13
      snap(12, 129, 0, [[129, 74]], 1); // ack is still 1, but tick 12 has already passed on the server
      expect(correction).toHaveLastReturnedWith('snap');
      expect(correction.mock.lastCall?.[2].map((e) => e.tick)).toEqual([13]);
      expect(v.state[0]).toBeCloseTo(74);
      expect(v.offset).toEqual({ x: 0, y: 0, z: 0 });
    } finally {
      game.dispose();
    }
  });
});
