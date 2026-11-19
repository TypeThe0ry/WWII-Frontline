import { describe, expect, it } from 'vitest';
import type { WasmPredictor } from '../net/wasm';
import { VehiclePredictor } from './vehicle-predict';

/** Stub: the vehicle moves +1 m in z per step from the reset position. */
const stub = (): WasmPredictor => {
  let z = 0;
  const out = new Float32Array(12);
  return {
    resetVehicle: (_id: string, kin: Float32Array) => void (z = kin[2] as number),
    clearVehicle: () => undefined,
    stepVehicle: () => {
      z += 1;
      out[2] = z;
      return out;
    },
  } as unknown as WasmPredictor;
};
const kin = (z: number): Float32Array => Float32Array.from([0, 0, z, 0, 0, 0, 0, 0, 0, 1, 0]);
const cmd = new Uint8Array(18);

describe('VehiclePredictor', () => {
  it('does not step before enter and after leave', () => {
    const v = new VehiclePredictor(stub());
    expect(v.step(1, cmd)).toBe(false);
    v.enter('t', 0, kin(0));
    expect(v.step(0, cmd)).toBe(false); // already simulated by the server
    expect(v.step(1, cmd)).toBe(true);
    v.leave();
    expect(v.step(2, cmd)).toBe(false);
  });

  it('classifies the reconcile error against 0.05 m / 1.0 m', () => {
    const v = new VehiclePredictor(stub());
    v.enter('t', 0, kin(0));
    v.step(1, cmd);
    v.step(2, cmd);
    const pending = [{ tick: 2, cmd }];
    expect(v.reconcile(1, kin(1.01), pending)).toBe('none');
    expect(v.state[2]).toBe(2);
    expect(v.reconcile(1, kin(1.3), pending)).toBe('blend');
    expect(v.offset.z).toBeCloseTo(-0.3, 4);
    v.update(0.125);
    expect(v.offset.z).toBeCloseTo(-0.15, 4);
    expect(v.reconcile(1, kin(5), pending)).toBe('snap');
    expect(v.state[2]).toBe(6);
    expect(v.offset.z).toBe(0);
  });

  it('view adds the remaining blend offset', () => {
    const v = new VehiclePredictor(stub());
    v.enter('t', 0, kin(0));
    v.step(1, cmd);
    v.reconcile(0, kin(0.5), [{ tick: 1, cmd }]); // prev 1, new 1.5 -> offset -0.5
    const p = { x: 0, y: 0, z: 0 };
    v.view(p);
    expect(p.z).toBeCloseTo(1, 5);
  });
});
