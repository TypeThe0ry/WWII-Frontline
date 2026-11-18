import { describe, expect, it } from 'vitest';
import { RemoteVehicleStore, VEHICLE_STRIDE, type VehiclePose } from './vehicle-store';

const row = (x: number, z: number, vz = 0, qw = 1): Float32Array => {
  const s = new Float32Array(VEHICLE_STRIDE);
  s[0] = x;
  s[2] = z;
  s[5] = vz;
  s[9] = qw;
  return s;
};
const pose = (): VehiclePose => ({ x: 0, y: 0, z: 0, qx: 0, qy: 0, qz: 0, qw: 1, turretYaw: 0, gunPitch: 0 });
const meta = (seats = 1): Uint8Array => Uint8Array.from([seats, 0, 255, 0]);

describe('RemoteVehicleStore', () => {
  it('reports new ids once and ignores stale ticks', () => {
    const st = new RemoteVehicleStore();
    expect(st.push(10, 0, Uint8Array.of(130), row(0, 0), meta())).toEqual([130]);
    expect(st.push(11, 50, Uint8Array.of(130), row(0, 1), meta())).toEqual([]);
    st.push(11, 60, Uint8Array.of(130), row(9, 9), meta()); // duplicate tick
    const p = pose();
    st.sample(130, 11, 33.3, p);
    expect(p.z).toBe(1);
  });

  it('interpolates between samples', () => {
    const st = new RemoteVehicleStore();
    st.push(10, 0, Uint8Array.of(130), row(0, 0), meta());
    st.push(12, 0, Uint8Array.of(130), row(0, 4), meta());
    const p = pose();
    expect(st.sample(130, 11, 33.3, p)).toBe(true);
    expect(p.z).toBeCloseTo(2, 5);
  });

  it('extrapolates with the sampled velocity for at most 150 ms', () => {
    const st = new RemoteVehicleStore();
    st.push(10, 0, Uint8Array.of(130), row(0, 0, 10), meta());
    const p = pose();
    st.sample(130, 12, 33.3, p); // 66.6 ms ahead
    expect(p.z).toBeCloseTo(0.666, 2);
    st.sample(130, 40, 33.3, p); // far ahead: capped at 150 ms
    expect(p.z).toBeCloseTo(1.5, 5);
  });

  it('holds across a teleport and expires silent tracks', () => {
    const st = new RemoteVehicleStore();
    st.push(10, 0, Uint8Array.of(130), row(0, 0), meta());
    st.push(11, 0, Uint8Array.of(130), row(500, 0), meta());
    const p = pose();
    st.sample(130, 10.5, 33.3, p);
    expect(p.x).toBe(0);
    const gone: number[] = [];
    st.expire(5000, 1500, (id) => gone.push(id));
    expect(gone).toEqual([130]);
    expect(st.has(130)).toBe(false);
  });

  it('normalises the interpolated quaternion', () => {
    const st = new RemoteVehicleStore();
    const a = row(0, 0);
    const b = row(0, 0);
    b[9] = 0;
    b[8] = 1; // 180 deg about z
    st.push(1, 0, Uint8Array.of(130), a, meta());
    st.push(3, 0, Uint8Array.of(130), b, meta());
    const p = pose();
    st.sample(130, 2, 33.3, p);
    expect(Math.hypot(p.qx, p.qy, p.qz, p.qw)).toBeCloseTo(1, 5);
  });
});
