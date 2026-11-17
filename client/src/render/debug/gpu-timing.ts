import { EngineInstrumentation } from '@babylonjs/core/Instrumentation/engineInstrumentation';
import '@babylonjs/core/Engines/Extensions/engine.query';
import type { AbstractEngine } from '@babylonjs/core/Engines/abstractEngine';
import type { Capabilities } from '../capabilities';
import type { PerfDump } from './perf-export';

export type GpuReading = PerfDump['gpu_ms'];

/**
 * GPU frame time via Babylon's EngineInstrumentation (8.56: `captureGPUFrameTime` +
 * `gpuFrameTimeCounter`, values in nanoseconds). WebGPU needs the `timestamp-query` device feature,
 * WebGL2 needs EXT_disjoint_timer_query_webgl2; otherwise the reading is an interval-based estimate.
 */
export class GpuTimer {
  readonly supported: boolean;
  readonly source: 'timestamp-query' | 'disjoint-timer-query' | null;
  readonly unsupportedReason: string | null;
  private readonly instr: EngineInstrumentation | null;

  constructor(
    engine: AbstractEngine,
    kind: 'webgpu' | 'webgl2',
    caps: Capabilities,
  ) {
    if (kind === 'webgpu') {
      const feats = (engine as unknown as { enabledExtensions?: readonly string[] }).enabledExtensions ?? [];
      this.supported = feats.includes('timestamp-query');
      this.source = this.supported ? 'timestamp-query' : null;
      this.unsupportedReason = this.supported
        ? null
        : caps.webgpu.timestampQuery
          ? 'timestamp-query not enabled on device'
          : 'adapter lacks timestamp-query (browser flag or hardware)';
    } else {
      this.supported = caps.webgl2.timerQuery;
      this.source = this.supported ? 'disjoint-timer-query' : null;
      this.unsupportedReason = this.supported ? null : 'EXT_disjoint_timer_query_webgl2 unavailable';
    }
    this.instr = this.supported ? new EngineInstrumentation(engine) : null;
    if (this.instr) this.instr.captureGPUFrameTime = true;
  }

  /** Latest GPU frame time in ms; falls back to the frame-interval estimate when not measurable. */
  read(intervalMeanMs: number): GpuReading {
    if (this.instr && this.source) {
      const ns = this.instr.gpuFrameTimeCounter.current;
      if (ns > 0) return { value: ns / 1e6, source: this.source, reason: null };
      return { value: null, source: 'n/a', reason: 'no GPU sample yet' };
    }
    return intervalMeanMs > 0
      ? { value: intervalMeanMs, source: 'interval-estimate', reason: this.unsupportedReason }
      : { value: null, source: 'n/a', reason: this.unsupportedReason };
  }

  dispose(): void {
    this.instr?.dispose();
  }
}
