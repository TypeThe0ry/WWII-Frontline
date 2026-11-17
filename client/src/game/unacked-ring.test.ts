import { describe, expect, it } from 'vitest';
import { RING_CAPACITY, UnackedRing, seqDiff } from './unacked-ring';

const fill = (r: UnackedRing, seq: number): void => {
  const e = r.push();
  e.seq = seq & 0xffff;
  e.tick = seq;
  e.x = seq;
};

describe('UnackedRing', () => {
  it('capacity is 128; overflow drops the oldest (it becomes the base) and counts, without throwing', () => {
    const r = new UnackedRing();
    const dropped: number[] = [];
    r.onOverflow = (e) => dropped.push(e.seq);
    expect(r.capacity).toBe(RING_CAPACITY);
    r.clear(0);
    for (let s = 1; s <= 128; s++) fill(r, s);
    expect(r.length).toBe(128);
    expect(r.overflows).toBe(0);
    fill(r, 129);
    fill(r, 130);
    expect(r.length).toBe(128);
    expect(r.overflows).toBe(2);
    expect(dropped).toEqual([1, 2]);
    expect(r.base.seq).toBe(2);
    expect(r.at(0).seq).toBe(3);
    expect(r.newest()?.seq).toBe(130);
    expect(r.classify(1)).toBe('stale');
    expect(r.classify(2)).toBe('base');
    expect(r.classify(3)).toBe('entry');
    expect(r.classify(131)).toBe('future');
  });

  it('seq wraps 65535 -> 0 correctly (ack, classification, discard)', () => {
    const r = new UnackedRing();
    r.clear(65530);
    for (let i = 1; i <= 12; i++) fill(r, (65530 + i) & 0xffff); // 65531..65535, 0..6
    expect(r.at(0).seq).toBe(65531);
    expect(r.at(5).seq).toBe(0);
    expect(r.classify(65535)).toBe('entry');
    expect(r.classify(0)).toBe('entry');
    expect(r.classify(7)).toBe('future');
    expect(r.classify(65529)).toBe('stale');
    expect(r.discardUpTo(1)).toBe(true); // across the wrap
    expect(r.base.seq).toBe(1);
    expect(r.at(0).seq).toBe(2);
    expect(r.length).toBe(5);
    expect(r.discardUpTo(65000)).toBe(false);
    expect(r.stateAt(4)?.seq).toBe(4);
    expect(r.stateAt(99)).toBeUndefined();
  });

  it('discardUpTo(base) is a no-op, discarding everything leaves an empty ring with that base', () => {
    const r = new UnackedRing();
    r.clear(0);
    for (let s = 1; s <= 5; s++) fill(r, s);
    expect(r.discardUpTo(0)).toBe(true);
    expect(r.length).toBe(5);
    expect(r.discardUpTo(5)).toBe(true);
    expect(r.length).toBe(0);
    expect(r.base.seq).toBe(5);
    expect(r.base.x).toBe(5);
    expect(r.classify(5)).toBe('base');
    expect(r.classify(6)).toBe('future');
  });

  it('seqDiff is the signed u16 distance', () => {
    expect(seqDiff(1, 65535)).toBe(2);
    expect(seqDiff(65535, 1)).toBe(-2);
    expect(seqDiff(100, 100)).toBe(0);
  });
});
