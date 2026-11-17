import { describe, expect, it } from 'vitest';
import { crc8, predHash, quantizePosition } from './pred-hash';

describe('pred_hash (CRC-8 poly 0x07 over the wire-quantised position)', () => {
  it('CRC-8 check value of "123456789" is 0xF4 (standard CRC-8/SMBUS)', () => {
    expect(crc8(new TextEncoder().encode('123456789'))).toBe(0xf4);
  });
  // Golden values for the server side (computed independently in Python, see PLAN_ISSUES PI-023).
  it('golden positions', () => {
    expect(predHash(0, 0, 0)).toBe(0);
    expect(predHash(128, 2.4, 56)).toBe(66);
    expect(predHash(100.40625, 12, 300)).toBe(175);
    expect(predHash(255.97, 200, 3.5)).toBe(151);
    expect(predHash(-5, 300, 1e9)).toBe(16); // clamped like quant_pos_xz / quant_pos_y
  });
  it('positions inside one 1/32 m cell hash alike, neighbouring cells (usually) do not', () => {
    expect(predHash(10.001, 2, 20.001)).toBe(predHash(10.0, 2, 20.0));
    expect(predHash(10.04, 2, 20)).not.toBe(predHash(10.0, 2, 20));
    expect(quantizePosition(10.04, 'xz')).toBeCloseTo(10.03125, 6);
  });
  it('non-finite input hashes as 0 codes instead of throwing', () => {
    expect(predHash(NaN, Infinity, -Infinity)).toBe(predHash(0, 0, 0));
  });
});
