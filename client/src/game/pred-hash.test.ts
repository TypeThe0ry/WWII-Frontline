import { describe, expect, it } from 'vitest';
import { HAVE_WASM, loadWasm } from '../net/test-support';

// `pred_hash` has one implementation (Rust, crates/protocol/src/snapshot/pred_hash.rs, exported by sim-wasm as
// `predHash`; PI-023). These golden values are the same ones the Rust tests pin, so a drift on either side fails.
describe.skipIf(!HAVE_WASM)('pred_hash through sim-wasm (CRC-8 poly 0x07 over the wire-quantised position)', () => {
  it('golden positions (same values as the Rust tests)', async () => {
    const w = await loadWasm();
    expect(w.predHash(0, 0, 0)).toBe(0);
    expect(w.predHash(128, 2.4, 56)).toBe(66);
    expect(w.predHash(100.40625, 12, 300)).toBe(175);
    expect(w.predHash(255.97, 200, 3.5)).toBe(151);
    expect(w.predHash(-5, 300, 1e9)).toBe(16); // clamped like quant_pos_xz / quant_pos_y
  });
  it('positions inside one 1/32 m cell hash alike, neighbouring cells do not', async () => {
    const w = await loadWasm();
    expect(w.predHash(10.001, 2, 20.001)).toBe(w.predHash(10.0, 2, 20.0));
    expect(w.predHash(10.04, 2, 20)).not.toBe(w.predHash(10.0, 2, 20));
  });
  it('non-finite input hashes as 0 codes instead of throwing', async () => {
    const w = await loadWasm();
    expect(w.predHash(NaN, Infinity, -Infinity)).toBe(w.predHash(0, 0, 0));
  });
});
