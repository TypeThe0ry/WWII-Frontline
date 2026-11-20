import { expect, it } from 'vitest';
import { summarize } from './signal-benchmark';

it('uses the unrounded mean and nearest-rank P95, including zero-duration samples', () => {
  const samples = [10, ...Array<number>(19).fill(0)];
  expect(summarize(samples)).toEqual({ meanMs: 0.5, p95Ms: 0 });
  expect(samples[0]).toBe(10);
});

it.each([[], [NaN], [Infinity], [-1]])('rejects invalid timing samples %#', (...values: number[]) => {
  expect(() => summarize(values)).toThrow();
});
