/**
 * Per-frame GPU upload budget (ARCHITECTURE 9.3): at most `maxMs` of upload time or `maxBytes` per frame, whichever is hit first.
 * Jobs run in priority order (lower number first, FIFO inside a priority); a job that does not fit waits for the next frame,
 * except the first job of a frame, which always runs so an oversized job cannot starve.
 */
export interface UploadJob {
  bytes: number;
  prio?: number;
  run(): void;
}

export interface FrameStats {
  ms: number;
  bytes: number;
  jobs: number;
}

export class UploadScheduler {
  readonly #queue: { job: UploadJob; seq: number }[] = [];
  #seq = 0;
  readonly frames: FrameStats[] = [];
  constructor(readonly maxMs = 1.5, readonly maxBytes = 2 * 1024 * 1024, private readonly now: () => number = () => performance.now()) {}

  get pending(): number {
    return this.#queue.length;
  }

  enqueue(job: UploadJob): void {
    this.#queue.push({ job, seq: this.#seq++ });
    this.#queue.sort((a, b) => (a.job.prio ?? 5) - (b.job.prio ?? 5) || a.seq - b.seq);
  }

  /** Call once per frame (before render). Returns the stats of this frame. */
  runFrame(): FrameStats {
    const t0 = this.now();
    let bytes = 0;
    let jobs = 0;
    while (this.#queue.length > 0) {
      const next = this.#queue[0]!.job;
      if (jobs > 0 && (bytes + next.bytes > this.maxBytes || this.now() - t0 >= this.maxMs)) break;
      this.#queue.shift();
      next.run();
      bytes += next.bytes;
      jobs++;
    }
    const s = { ms: this.now() - t0, bytes, jobs };
    if (jobs > 0) this.frames.push(s);
    return s;
  }
}
