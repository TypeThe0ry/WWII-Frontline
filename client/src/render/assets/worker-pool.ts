/**
 * Task pool with priorities and cancellation. A "worker" is any async function (a real `Worker` wrapper in the browser, an
 * in-memory fake in tests); the pool size is `min(4, hardwareConcurrency - 2)` (ARCHITECTURE 9.3).
 */
export function defaultPoolSize(hardwareConcurrency: number = (globalThis.navigator?.hardwareConcurrency ?? 4)): number {
  return Math.max(1, Math.min(4, hardwareConcurrency - 2));
}

export class CancelledError extends Error {
  constructor() {
    super('cancelled');
    this.name = 'CancelledError';
  }
}

export interface Task<T> {
  readonly promise: Promise<T>;
  cancel(): void;
}

interface Pending<I, O> {
  input: I;
  prio: number;
  seq: number;
  resolve(o: O): void;
  reject(e: unknown): void;
  cancelled: boolean;
}

export class WorkerPool<I, O> {
  readonly #queue: Pending<I, O>[] = [];
  #running = 0;
  #seq = 0;
  constructor(private readonly work: (input: I) => Promise<O>, readonly size: number = defaultPoolSize()) {}

  get inflight(): number {
    return this.#running + this.#queue.length;
  }

  submit(input: I, prio = 5): Task<O> {
    let p!: Pending<I, O>;
    const promise = new Promise<O>((resolve, reject) => {
      p = { input, prio, seq: this.#seq++, resolve, reject, cancelled: false };
    });
    this.#queue.push(p);
    this.#queue.sort((a, b) => a.prio - b.prio || a.seq - b.seq);
    queueMicrotask(() => this.#pump());
    return {
      promise,
      cancel: () => {
        if (p.cancelled) return;
        p.cancelled = true;
        const i = this.#queue.indexOf(p);
        if (i >= 0) {
          this.#queue.splice(i, 1);
          p.reject(new CancelledError());
        }
        // Already running tasks finish but their result is dropped by the caller.
      },
    };
  }

  #pump(): void {
    while (this.#running < this.size && this.#queue.length > 0) {
      const p = this.#queue.shift()!;
      this.#running++;
      this.work(p.input).then(
        (o) => (p.cancelled ? p.reject(new CancelledError()) : p.resolve(o)),
        (e) => p.reject(e),
      ).finally(() => {
        this.#running--;
        this.#pump();
      });
    }
  }
}
