import { describe, it, expect } from 'vitest';
import { firstScreenShards, firstScreenBytes, checkBudget, needsMoreShards, applyFallbackFlag, type FontFile } from './loader';

const files: FontFile[] = [
  { file: 'l1', bytes: 55236, shard: 'latin' },
  { file: 'c0', bytes: 60028, shard: 'cjk0' },
  { file: 'c1', bytes: 319900, shard: 'cjk1' },
];

describe('font loader policy', () => {
  it('first screen: Latin only for en; Latin + cjk0 for zh-Hans (cjk1+ are on demand)', () => {
    expect(firstScreenShards('en')).toEqual(['latin']);
    expect(firstScreenShards('zh-Hans')).toEqual(['latin', 'cjk0']);
    expect(firstScreenBytes(files, 'en')).toBe(55236);
    expect(firstScreenBytes(files, 'zh-Hans')).toBe(115264);
  });
  it('budget check passes for measured sizes and fails when exceeded', () => {
    expect(checkBudget(files)).toEqual([]);
    expect(checkBudget([{ file: 'x', bytes: 500 * 1024, shard: 'cjk0' }, ...files])).toHaveLength(1);
    expect(checkBudget([{ file: 'x', bytes: 300 * 1024, shard: 'latin' }])).toHaveLength(1);
  });
  it('more shards needed only for CJK chars outside the first-screen set', () => {
    const set = new Set([0x5f00, 0x59cb]);
    expect(needsMoreShards('Hello 开始', set)).toBe(false);
    expect(needsMoreShards('龘', set)).toBe(true);
  });
  it('fallback flag toggles the attribute', () => {
    const attrs = new Map<string, string>();
    const root = { setAttribute: (k: string, v: string) => void attrs.set(k, v), removeAttribute: (k: string) => void attrs.delete(k) };
    applyFallbackFlag(root, true);
    expect(attrs.has('data-font-fallback')).toBe(true);
    applyFallbackFlag(root, false);
    expect(attrs.has('data-font-fallback')).toBe(false);
  });
});
