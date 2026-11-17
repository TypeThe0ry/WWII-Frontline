import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { FIXTURES } from './helpers.ts';

describe('fixtures', () => {
  const lines = readFileSync(join(FIXTURES, 'SHA256SUMS'), 'utf8').trim().split('\n');
  it('SHA256SUMS lists every .gltf fixture and matches the files', () => {
    expect(lines.length).toBeGreaterThanOrEqual(5);
    for (const l of lines) {
      const [sum, name] = l.split(/\s+/) as [string, string];
      expect(createHash('sha256').update(readFileSync(join(FIXTURES, name))).digest('hex'), name).toBe(sum);
    }
  });
});
