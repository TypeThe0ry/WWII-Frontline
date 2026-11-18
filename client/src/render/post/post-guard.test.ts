import { describe, expect, it } from 'vitest';
import { PostBudgetGuard } from './post-guard';

describe('PostBudgetGuard', () => {
  it('stays quiet at 60 fps and does not trigger before a full window', () => {
    const g = new PostBudgetGuard(25, 90);
    let fired = false;
    for (let i = 0; i < 1000; i++) fired = g.observe(16.7) || fired;
    expect(fired).toBe(false);
    const g2 = new PostBudgetGuard(25, 90);
    for (let i = 0; i < 89; i++) expect(g2.observe(80)).toBe(false); // slow, but the window is not full yet
    expect(g2.observe(80)).toBe(true);
  });

  it('a few hitches inside the window do not trigger (P95), sustained slowness does, and only once', () => {
    const g = new PostBudgetGuard(25, 100);
    let fired = 0;
    for (let i = 0; i < 400; i++) if (g.observe(i % 40 === 0 ? 90 : 16.7)) fired++; // 2.5 % of frames at 90 ms
    expect(fired).toBe(0);
    for (let i = 0; i < 100; i++) if (g.observe(i % 10 === 0 ? 16.7 : 40)) fired++; // 90 % of frames at 40 ms
    expect(fired).toBe(1);
    for (let i = 0; i < 100; i++) if (g.observe(100)) fired++;
    expect(fired).toBe(1);
  });
});
