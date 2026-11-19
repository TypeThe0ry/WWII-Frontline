import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { expect, it } from 'vitest';
import { ChunkIndex } from './chunk-stream-source';
import { ChunkSlots, MEDIUM_CHUNK_SLOTS, ULTRA_CHUNK_SLOTS } from './chunk-slots';

it('indexes the checked-in M1-035 Carentan bake without wrapping out-of-grid queries', () => {
  const path = fileURLToPath(new URL('../../../../data/maps/carentan/m1/chunks/index.json', import.meta.url));
  const raw = JSON.parse(readFileSync(path, 'utf8'));
  const index = new ChunkIndex(raw);
  expect(raw.count).toBe(380);
  expect(index.get(5, 8)).toMatchObject({ key: 'terrain_carentan/b_5_8', files: ['b_5_8.fchk', 'c_5_8.fcol'] });
  expect(index.get(24, 26)).toBeDefined();
  expect(index.get(4, 8)).toBeUndefined();
  expect(index.get(37, 7)).toBeUndefined();
  expect(() => new ChunkIndex({ ...raw, count: 2, chunks: [raw.chunks[0], raw.chunks[0]] })).toThrow('Duplicate chunk');
});

it('holds stable texture-array layers and recycles freed slots without double release', () => {
  expect(MEDIUM_CHUNK_SLOTS).toBe(81);
  expect(ULTRA_CHUNK_SLOTS).toBe(169);
  const slots = new ChunkSlots(2);
  expect(slots.acquire(1, 2)).toBe(0);
  expect(slots.acquire(1, 2)).toBe(0);
  expect(slots.acquire(2, 2)).toBe(1);
  expect(slots.acquire(3, 2)).toBeUndefined();
  slots.release(1, 2);
  slots.release(1, 2);
  expect(slots.get(1, 2)).toBeUndefined();
  expect(slots.acquire(3, 2)).toBe(0);
  expect(slots.acquire(4, 2)).toBeUndefined();
  expect(slots.get(2, 2)).toBe(1);
  expect(slots.get(34, 1)).toBeUndefined();
  slots.release(34, 1);
  expect(slots.get(2, 2)).toBe(1);
});
