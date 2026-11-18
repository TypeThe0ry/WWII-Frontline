import { describe, expect, it } from 'vitest';
import { SeatMachine, SWAP_MOVING_TICKS } from './seat';
import { extrapolate, secondDiffRms } from './vehicle-interp';

describe('SeatMachine', () => {
  it('enter -> confirm -> swap -> exit', () => {
    const m = new SeatMachine();
    expect(m.requestEnter(3, 'driver')).toBe(true);
    expect(m.state.pending).toBe('enter');
    m.confirm('driver', 3);
    expect(m.predictsVehicle).toBe(true);
    expect(m.requestSwap('gunner', 10, 0)).toBe(true);
    expect(m.due(10)).toBe(true); // stopped: immediate
    m.confirm('gunner', 3);
    expect(m.predictsVehicle).toBe(false);
    expect(m.requestExit()).toBe(true);
    m.confirm('none');
    expect(m.state).toEqual({ seat: 'none' });
  });

  it('swap while moving takes 2 s (60 ticks), exactly', () => {
    const m = new SeatMachine();
    m.confirm('driver', 1);
    m.requestSwap('commander', 100, 30);
    expect(m.due(100 + SWAP_MOVING_TICKS - 1)).toBe(false);
    expect(m.due(100 + SWAP_MOVING_TICKS)).toBe(true);
    expect(SeatMachine.swapDelay(4.99)).toBe(0);
    expect(SeatMachine.swapDelay(5)).toBe(60);
    expect(SeatMachine.swapDelay(-12)).toBe(60);
  });

  it('illegal requests are refused and change nothing', () => {
    const m = new SeatMachine();
    expect(m.requestExit()).toBe(false);
    expect(m.requestSwap('gunner', 0, 0)).toBe(false);
    m.confirm('driver', 1);
    expect(m.requestEnter(2, 'gunner')).toBe(false);
    expect(m.requestSwap('driver', 0, 0)).toBe(false); // same seat
    m.requestSwap('gunner', 0, 0);
    expect(m.requestSwap('commander', 0, 0)).toBe(false); // already pending
    expect(m.refused).toBe(5);
  });

  it('server rejection restores the previous seat', () => {
    const m = new SeatMachine();
    m.confirm('driver', 7);
    m.requestSwap('gunner', 0, 0);
    m.reject();
    expect(m.state).toEqual({ seat: 'driver', vehicleId: 7 });
    const e = new SeatMachine();
    e.requestEnter(1, 'driver');
    e.reject();
    expect(e.state).toEqual({ seat: 'none' });
  });
});

describe('vehicle-interp', () => {
  it('extrapolation is capped at 150 ms', () => {
    expect(extrapolate(10, 8, 100)).toBeCloseTo(10.8);
    expect(extrapolate(10, 8, 1000)).toBeCloseTo(11.2);
    expect(extrapolate(10, 8, -5)).toBe(10);
  });
  it('second-difference RMS: straight line 0, jitter > 0', () => {
    expect(secondDiffRms([0, 1, 2, 3, 4])).toBeCloseTo(0);
    expect(secondDiffRms([0, 1, 0, 1, 0])).toBeGreaterThan(1);
    expect(secondDiffRms([1])).toBe(0);
  });
});
