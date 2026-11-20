import { describe, it, expect, vi } from 'vitest';
import { firstScreenShards, firstScreenBytes, checkBudget, needsMoreShards, applyFallbackFlag, installUiFonts, type FontFile } from './loader';

const files: FontFile[] = [
  ...[1, 2, 3, 4].map(i => ({ file: `l${i}`, bytes: 1000, br_bytes: 900, shard: 'latin' as const })),
  { file: 'c0', bytes: 20000, br_bytes: 19000, shard: 'cjk0' },
  { file: 'c1', bytes: 319900, br_bytes: 319000, shard: 'cjk1' },
];

describe('font loader policy', () => {
  it('first screen: Latin only for en; Latin + cjk0 for zh-Hans (cjk1+ are on demand)', () => {
    expect(firstScreenShards('en')).toEqual(['latin']);
    expect(firstScreenShards('zh-Hans')).toEqual(['latin', 'cjk0']);
    expect(firstScreenBytes(files, 'en')).toBe(4000);
    expect(firstScreenBytes(files, 'zh-Hans')).toBe(24000);
    expect(firstScreenBytes(files, 'zh-Hans', 'br_bytes')).toBe(22600);
  });
  it('checks complete first-screen sets against Brotli limits, including equality', () => {
    expect(checkBudget(files)).toEqual([]);
    const edge = files.map(f => ({ ...f, br_bytes: f.shard === 'latin' ? 51200 : 204800 }));
    expect(checkBudget(edge)).toEqual([]);
    expect(checkBudget(edge.map(f => ({ ...f, br_bytes: f.br_bytes + 1 })))).toHaveLength(2);
    expect(checkBudget(files.filter(f => f.shard !== 'latin'))).not.toEqual([]);
    expect(checkBudget([...files, files[0]!])).not.toEqual([]);
    expect(checkBudget(files.map(f => ({ ...f, br_bytes: NaN })))).not.toEqual([]);
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
  it('flags stylesheet and UI face failures without fetching shards eagerly; detaches on cleanup', () => {
    const root = { setAttribute: vi.fn(), removeAttribute: vi.fn() };
    const link = Object.assign(new EventTarget(), { remove: vi.fn(), rel: '', href: '' });
    const fonts = new EventTarget();
    const append = vi.fn();
    const dispose = installUiFonts({ documentElement: root, createElement: () => link, fonts, head: { append } } as unknown as Document);
    expect(append).toHaveBeenCalledWith(link);
    expect(link.rel).toBe('stylesheet');
    expect(link.href).toBe('/fonts/fonts.css');
    const error = (family: string) => Object.assign(new Event('loadingerror'), { fontfaces: [{ family }] });
    fonts.dispatchEvent(error('unrelated'));
    expect(root.setAttribute).not.toHaveBeenCalled();
    fonts.dispatchEvent(error('"Noto Sans SC"'));
    link.dispatchEvent(new Event('error'));
    expect(root.setAttribute).toHaveBeenCalledTimes(2);
    dispose();
    fonts.dispatchEvent(error('Noto Sans SC'));
    expect(root.setAttribute).toHaveBeenCalledTimes(2);
    expect(link.remove).toHaveBeenCalledOnce();
  });
});
