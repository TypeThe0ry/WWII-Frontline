/**
 * `pred_hash` (ARCHITECTURE 6.5.3): CRC-8 over the wire-quantised position, so that the server can tell
 * "the client's prediction agrees with me" from one byte and only then omit `pos_corr`.
 *
 * Definition (shared contract with the server, PLAN_ISSUES PI-021; the task card asks for a sim-wasm
 * `predHash`, which does not exist yet, so the function lives here and `pred-hash.test.ts` pins golden
 * values for the server side to reproduce):
 *   codes  x = clamp(round(x * 32), 0, 65535)   (protocol::quant::quant_pos_xz, step 1/32 m)
 *          y = clamp(round(y * 32), 0, 8191)    (quant_pos_y, 13 bit)
 *          z = clamp(round(z * 32), 0, 65535)
 *   bytes  x lo, x hi, y lo, y hi, z lo, z hi   (little endian)
 *   crc    CRC-8, polynomial 0x07, init 0x00, no reflection, no final xor
 */
const POS_STEP_INV = 32;
const MAX_XZ = 0xffff;
const MAX_Y = 0x1fff;

const CRC_TABLE = (() => {
  const t = new Uint8Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let b = 0; b < 8; b++) c = c & 0x80 ? ((c << 1) ^ 0x07) & 0xff : (c << 1) & 0xff;
    t[i] = c;
  }
  return t;
})();

export function crc8(bytes: ArrayLike<number>): number {
  let c = 0;
  for (let i = 0; i < bytes.length; i++) c = CRC_TABLE[(c ^ (bytes[i] as number)) & 0xff] as number;
  return c;
}

const q = (v: number, max: number): number => {
  if (!Number.isFinite(v)) return 0;
  // Same rounding as Rust `(v / step + 0.5).clamp(0, max) as u16` for non-negative values.
  return Math.min(max, Math.max(0, Math.floor(Math.fround(Math.fround(v) * POS_STEP_INV + 0.5))));
};

const scratch = new Uint8Array(6);

/** Quantises a position (metres) to the wire codes and returns the CRC-8. Allocation free. */
export function predHash(x: number, y: number, z: number): number {
  const cx = q(x, MAX_XZ);
  const cy = q(y, MAX_Y);
  const cz = q(z, MAX_XZ);
  scratch[0] = cx & 0xff;
  scratch[1] = cx >> 8;
  scratch[2] = cy & 0xff;
  scratch[3] = cy >> 8;
  scratch[4] = cz & 0xff;
  scratch[5] = cz >> 8;
  return crc8(scratch);
}

/** Position after the wire round trip (what `pos_corr` carries). */
export function quantizePosition(v: number, kind: 'xz' | 'y'): number {
  return q(v, kind === 'y' ? MAX_Y : MAX_XZ) / POS_STEP_INV;
}
