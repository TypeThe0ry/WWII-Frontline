import { batch, signal, type Signal } from '@preact/signals';
import { render } from 'preact';
import { flushSync } from 'preact/compat';

const SIGNALS = 100;
const WARMUP = 30;
const SAMPLES = 120;

export function summarize(samples: readonly number[]) {
  if (!samples.length || samples.some(ms => !Number.isFinite(ms) || ms < 0)) throw new Error('Invalid UI timing samples');
  const sorted = [...samples].sort((a, b) => a - b);
  return {
    meanMs: samples.reduce((total, ms) => total + ms, 0) / samples.length,
    p95Ms: sorted[Math.ceil(samples.length * 0.95) - 1]!,
  };
}

function Cell({ value }: { value: Signal<number> }) {
  return <span>{value.value}</span>;
}

/** One synchronous batch per rAF. Time includes writes, component renders and DOM commits;
 * excludes setup, DOM verification, style, layout, paint, and inter-frame wait. */
export type SignalResult = ReturnType<typeof summarize> & {
  frames: number; signals: number; warmup: number; nodes: number;
};

export function benchmarkSignals(root: Element, done: (result: SignalResult) => void, fail: (message: string) => void) {
  const values = Array.from({ length: SIGNALS }, () => signal(0));
  const host = document.createElement('div');
  host.hidden = true;
  root.append(host);
  render(<>{values.map((value, i) => <Cell key={i} value={value} />)}</>, host);
  const cells = [...host.children];
  const samples: number[] = [];
  let frame = 0;
  let raf = requestAnimationFrame(step);
  function stop() {
    cancelAnimationFrame(raf);
    if (!host.parentNode) return;
    render(null, host);
    host.remove();
  }
  function step() {
    try {
      frame++;
      const start = performance.now();
      flushSync(() => batch(() => { for (const value of values) value.value = frame; }));
      const elapsed = performance.now() - start;
      if (cells.length !== SIGNALS || cells.some(cell => cell.textContent !== String(frame))) {
        throw new Error(`Incomplete Preact commit at frame ${frame}`);
      }
      if (frame > WARMUP) samples.push(elapsed);
      if (samples.length < SAMPLES) raf = requestAnimationFrame(step);
      else {
        stop();
        done({ ...summarize(samples), frames: samples.length, signals: SIGNALS, warmup: WARMUP, nodes: root.querySelectorAll('*').length });
      }
    } catch (error) {
      stop();
      fail(String(error));
    }
  }
  return stop;
}
