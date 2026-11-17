import { Engine } from '@babylonjs/core/Engines/engine';
import { WebGPUEngine } from '@babylonjs/core/Engines/webgpuEngine';
import type { AbstractEngine } from '@babylonjs/core/Engines/abstractEngine';
import type { Scene } from '@babylonjs/core/scene';
import { SceneInstrumentation } from '@babylonjs/core/Instrumentation/sceneInstrumentation';
import { probeCapabilities, errMsg, type Capabilities, type Gl2Like } from './capabilities';
import { createTestScene } from './test-scene';
import { FrameStats } from './debug/frame-stats';
import { GpuTimer } from './debug/gpu-timing';
import type { RenderSnapshot } from './debug/perf-export';

export type RendererPref = 'webgpu' | 'webgl2' | 'auto';
export type RendererKind = 'webgpu' | 'webgl2';

export interface RendererOptions {
  renderer: RendererPref;
  fly: boolean;
}

export interface RenderContext {
  engine: AbstractEngine;
  scene: Scene;
  /** The canvas actually in use (replaced by a fresh element if a failed WebGPU attempt tainted the original). */
  canvas: HTMLCanvasElement;
  kind: RendererKind;
  caps: Capabilities;
  /** Why the requested/auto-preferred WebGPU path was not used; null when no fallback happened. */
  fallbackReason: string | null;
  /** Register a callback run at the start of every render frame with real_dt in ms. */
  onFrame(cb: (realDtMs: number) => void): () => void;
  /** Start the render loop. */
  start(): void;
  snapshot(): RenderSnapshot;
  dispose(): void;
}

/** Hard cap on WebGPU init so a hung adapter cannot stall startup (fallback target is <= 2 s). */
const WEBGPU_INIT_TIMEOUT_MS = 1500;

export function parseRendererParam(v: string | null): RendererPref {
  return v === 'webgpu' || v === 'webgl2' ? v : 'auto';
}

function withTimeout<T>(p: Promise<T>, ms: number, what: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const t = setTimeout(() => reject(new Error(`${what} timed out after ${ms} ms`)), ms);
    p.then(
      (v) => {
        clearTimeout(t);
        resolve(v);
      },
      (e: unknown) => {
        clearTimeout(t);
        reject(e instanceof Error ? e : new Error(String(e)));
      },
    );
  });
}

function probeGl2(): Gl2Like | null {
  const c = document.createElement('canvas');
  const gl = c.getContext('webgl2');
  if (!gl) return null;
  // Snapshot what we need, then release the throwaway context.
  const exts = gl.getSupportedExtensions();
  const max = gl.getParameter(gl.MAX_TEXTURE_SIZE) as unknown;
  gl.getExtension('WEBGL_lose_context')?.loseContext();
  return { MAX_TEXTURE_SIZE: 0, getParameter: () => max, getSupportedExtensions: () => exts };
}

function freshCanvas(old: HTMLCanvasElement): HTMLCanvasElement {
  const c = document.createElement('canvas');
  for (const a of Array.from(old.attributes)) c.setAttribute(a.name, a.value);
  old.replaceWith(c);
  return c;
}

export async function createRenderer(canvas: HTMLCanvasElement, opts: RendererOptions): Promise<RenderContext> {
  const caps = await probeCapabilities(navigator, location.search.includes('nogl') ? undefined : probeGl2);
  let fallbackReason: string | null = null;
  let engine: AbstractEngine | null = null;
  let kind: RendererKind = 'webgl2';
  let tainted = false;

  if (opts.renderer !== 'webgl2') {
    try {
      if (!caps.webgpu.adapter) throw new Error(caps.webgpu.error ?? 'no WebGPU adapter');
      if (!(await WebGPUEngine.IsSupportedAsync)) throw new Error('WebGPUEngine.IsSupportedAsync is false');
      tainted = true;
      const gpu = new WebGPUEngine(canvas, {
        antialias: true,
        adaptToDeviceRatio: true,
        deviceDescriptor: caps.webgpu.timestampQuery && !location.search.includes('nots') ? { requiredFeatures: ['timestamp-query'] } : undefined,
      });
      try {
        await withTimeout(gpu.initAsync(), WEBGPU_INIT_TIMEOUT_MS, 'WebGPUEngine.initAsync');
      } catch (e) {
        gpu.dispose();
        throw e;
      }
      engine = gpu;
      kind = 'webgpu';
    } catch (e) {
      fallbackReason = errMsg(e);
      engine = null;
      console.warn(`[render] WebGPU unavailable, falling back to WebGL2: ${fallbackReason}`);
    }
  }

  if (!engine) {
    const target = tainted ? freshCanvas(canvas) : canvas;
    canvas = target;
    const gl = new Engine(
      target,
      true,
      { stencil: false, powerPreference: 'high-performance', preserveDrawingBuffer: false },
      true,
    );
    if (gl.webGLVersion < 2) {
      gl.dispose();
      throw new Error('WebGL2 is not available (Babylon created a WebGL1 context)');
    }
    engine = gl;
    kind = 'webgl2';
  }

  const eng = engine;
  const scene = createTestScene(eng, canvas, { fly: opts.fly });
  const frameStats = new FrameStats();
  const gpuTimer = new GpuTimer(eng, kind, caps);
  const sceneInstr = new SceneInstrumentation(scene);
  const callbacks: ((dt: number) => void)[] = [];

  (window as unknown as { __caps?: unknown }).__caps = {
    kind,
    fallbackReason,
    gpuTimerSource: gpuTimer.source,
    ...caps,
  };

  const onResize = () => eng.resize();
  window.addEventListener('resize', onResize);

  const ctx: RenderContext = {
    engine: eng,
    scene,
    canvas,
    kind,
    caps,
    fallbackReason,
    onFrame(cb) {
      callbacks.push(cb);
      return () => {
        const i = callbacks.indexOf(cb);
        if (i >= 0) callbacks.splice(i, 1);
      };
    },
    start() {
      eng.runRenderLoop(() => {
        const t0 = performance.now();
        const dt = frameStats.begin(t0);
        for (const cb of callbacks) cb(dt);
        scene.render();
        frameStats.end(performance.now() - t0);
      });
    },
    snapshot() {
      const interval = frameStats.interval.stats();
      return {
        renderer: kind,
        fallbackReason,
        frames: frameStats.frames,
        fps: frameStats.fps(),
        cpu: frameStats.cpu.stats(),
        interval,
        gpu: gpuTimer.read(interval.mean),
        drawCalls: sceneInstr.drawCallsCounter.current,
        activeMeshes: scene.getActiveMeshes().length,
        triangles: Math.round(scene.getActiveIndices() / 3),
        renderScale: 1 / eng.getHardwareScalingLevel(),
        width: eng.getRenderWidth(),
        height: eng.getRenderHeight(),
      };
    },
    dispose() {
      window.removeEventListener('resize', onResize);
      eng.stopRenderLoop();
      gpuTimer.dispose();
      sceneInstr.dispose();
      scene.dispose();
      eng.dispose();
    },
  };
  return ctx;
}
