import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import * as L from './layout';

// Layout constants equal the Rust golden (crates/sim-wasm/tests/golden/layout.json).
describe('wasm layout constants', () => {
  it('match the sim-wasm golden', () => {
    const golden = JSON.parse(readFileSync(fileURLToPath(new URL('../../../../crates/sim-wasm/tests/golden/layout.json', import.meta.url)), 'utf8')) as Record<string, number>;
    const mine = L as unknown as Record<string, number>;
    for (const [k, v] of Object.entries(golden)) expect(mine[`LAYOUT_${k}`], k).toBe(v);
    expect(Object.keys(mine).length).toBe(Object.keys(golden).length);
  });
});
