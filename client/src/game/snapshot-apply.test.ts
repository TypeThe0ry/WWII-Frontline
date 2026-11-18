import { describe, expect, it } from 'vitest';
import { EventBus, type GameEvents } from './event-bus';
import { EntityStore } from './interpolation';
import { SnapshotApplier } from './snapshot-apply';
import { FakeSnapshotDecoder } from './test-harness';
import { VEHICLE_STRIDE } from './vehicle-store';

describe('SnapshotApplier vehicles', () => {
  it('keeps vehicle ids (>= 128) out of the player store and feeds the vehicle store', () => {
    const bus = new EventBus<GameEvents>();
    const spawned: Array<[number, string]> = [];
    bus.on('entity.spawned', (e) => spawned.push([e.id, e.kind]));
    const store = new EntityStore();
    const ap = new SnapshotApplier(new FakeSnapshotDecoder(), store, bus);
    const veh = new Array<number>(VEHICLE_STRIDE).fill(0);
    veh[0] = 50;
    veh[9] = 1;
    const wire = FakeSnapshotDecoder.wire({
      tick: 5,
      ackInputSeq: 0,
      inputSlack: 0,
      self: null,
      ids: [3, 130],
      pos: [1, 0, 1, 50, 0, 60],
      yaw: [0, 0],
      pitch: [0, 0],
      removed: [],
      events: new Uint8Array(0),
      vehIds: [130],
      vehState: veh,
      vehMeta: [1, 0, 255, 0],
    });
    ap.decode(wire);
    ap.applyEntities(100);
    expect(store.alive[3]).toBe(1);
    expect(store.alive[130]).toBe(0);
    expect(ap.vehicles.has(130)).toBe(true);
    expect(spawned).toEqual([
      [3, 'player'],
      [130, 'vehicle'],
    ]);
    const gone = FakeSnapshotDecoder.wire({
      tick: 6,
      ackInputSeq: 0,
      inputSlack: 0,
      self: null,
      ids: [],
      pos: [],
      yaw: [],
      pitch: [],
      removed: [130],
      events: new Uint8Array(0),
    });
    ap.decode(gone);
    ap.applyEntities(150);
    expect(ap.vehicles.has(130)).toBe(false);
  });
});
