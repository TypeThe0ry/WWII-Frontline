import { describe, expect, it, vi } from 'vitest';
import { EventBus, ObjectPool } from './event-bus';

interface M {
  a: { n: number };
  b: { s: string };
}

describe('EventBus', () => {
  it('emit calls subscribers synchronously, in order, with the payload', () => {
    const bus = new EventBus<M>();
    const seen: string[] = [];
    bus.on('a', (p) => seen.push(`1:${p.n}`));
    bus.on('a', (p) => seen.push(`2:${p.n}`));
    bus.on('b', (p) => seen.push(`b:${p.s}`));
    bus.emit('a', { n: 7 });
    expect(seen).toEqual(['1:7', '2:7']); // already done when emit returns
    bus.emit('b', { s: 'x' });
    expect(seen).toEqual(['1:7', '2:7', 'b:x']);
  });

  it('a throwing subscriber does not break the others and is reported', () => {
    const bus = new EventBus<M>();
    const errors: unknown[] = [];
    bus.onError = (e, ev) => errors.push([e, ev]);
    const after = vi.fn();
    bus.on('a', () => {
      throw new Error('boom');
    });
    bus.on('a', after);
    bus.emit('a', { n: 1 });
    expect(after).toHaveBeenCalledTimes(1);
    expect(errors).toHaveLength(1);
    expect((errors[0] as [Error, string])[0].message).toBe('boom');
    expect((errors[0] as [Error, string])[1]).toBe('a');
  });

  it('unsubscribe stops delivery; on/off during emit are safe', () => {
    const bus = new EventBus<M>();
    const f = vi.fn();
    const off = bus.on('a', f);
    bus.emit('a', { n: 1 });
    off();
    off(); // idempotent
    bus.emit('a', { n: 2 });
    expect(f).toHaveBeenCalledTimes(1);
    expect(bus.listenerCount('a')).toBe(0);

    const order: string[] = [];
    const late = () => order.push('late');
    const self = (): void => {
      order.push('self');
      bus.off('a', self);
      bus.on('a', late);
    };
    bus.on('a', self);
    bus.emit('a', { n: 0 });
    expect(order).toEqual(['self']); // the subscriber added during emit starts with the next emit
    bus.emit('a', { n: 0 });
    expect(order).toEqual(['self', 'late']);
  });

  it('once fires a single time', () => {
    const bus = new EventBus<M>();
    const f = vi.fn();
    bus.once('b', f);
    bus.emit('b', { s: '1' });
    bus.emit('b', { s: '2' });
    expect(f).toHaveBeenCalledTimes(1);
  });

  it('ObjectPool reuses released objects', () => {
    const pool = new ObjectPool(() => ({ n: 0 }), (o) => (o.n = 0), 1);
    const o = pool.acquire();
    o.n = 5;
    pool.release(o);
    expect(pool.acquire()).toBe(o);
    expect(o.n).toBe(0);
  });

  it('hot path: 1e6 emits with a pooled payload allocate < 1 MB', () => {
    const bus = new EventBus<M>();
    const acc = new Int32Array(2); // typed array: a captured double would itself box a heap number per write
    bus.on('a', (p) => {
      acc[0] = (acc[0] ?? 0) + (p.n & 1023);
    });
    bus.on('a', (p) => {
      acc[1] = (acc[1] ?? 0) ^ p.n;
    });
    const pool = new ObjectPool(() => ({ n: 0 }), undefined, 4);
    const gc = (globalThis as { gc?: () => void }).gc;
    gc?.();
    const heap = (): number => (globalThis as unknown as { process: { memoryUsage(): { heapUsed: number } } }).process.memoryUsage().heapUsed;
    const before = heap();
    for (let i = 0; i < 1_000_000; i++) {
      const p = pool.acquire();
      p.n = i;
      bus.emit('a', p);
      pool.release(p);
    }
    gc?.();
    const delta = heap() - before;
    expect(acc[0]).not.toBe(0);
    // Without --expose-gc, garbage cannot be reclaimed between the two readings, so this is an upper bound on allocation.
    expect(delta).toBeLessThan(1024 * 1024);
  });
});
