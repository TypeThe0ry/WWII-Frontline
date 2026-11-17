import { FixedStepper } from './game';
import { createRenderer, parseRendererParam, DebugOverlay, buildPerfDump, installPerfApi } from './render';
import type { PerfApi } from './render';

declare global {
  interface Window {
    __perf?: PerfApi;
    __smoke?: unknown;
  }
}

/** Composition root: probe -> engine -> test scene -> loop. */
async function boot(): Promise<void> {
  const params = new URLSearchParams(location.search);
  const app = document.getElementById('app');
  if (!app) throw new Error('#app missing');
  const canvas = document.createElement('canvas');
  canvas.id = 'game-canvas';
  canvas.style.cssText = 'display:block;width:100vw;height:100vh;outline:none;touch-action:none';
  app.appendChild(canvas);

  const ctx = await createRenderer(canvas, {
    renderer: parseRendererParam(params.get('renderer')),
    fly: params.get('fly') === '1',
  });

  const overlay = new DebugOverlay(params.get('debug') === '1');
  const stepper = new FixedStepper();
  const getDump = () =>
    buildPerfDump(ctx.snapshot(), { steps_total: stepper.totalSteps, stepper_clamped: stepper.stepperClamped });
  installPerfApi(window, getDump);

  ctx.onFrame((dt) => {
    const { steps } = stepper.advance(dt);
    void steps; // game.update(steps) / render interpolation (alpha) are wired by later tasks.
    overlay.update(performance.now(), getDump());
  });
  ctx.start();

  if (params.get('smoke') === 'compute') {
    const { runComputeSmoke } = await import('./render/compute-smoke');
    window.__smoke = await runComputeSmoke(ctx);
  }
}

boot().catch((e: unknown) => {
  console.error('[boot] failed', e);
  const app = document.getElementById('app');
  if (app) app.textContent = `Boot failed: ${e instanceof Error ? e.message : String(e)}`;
});
