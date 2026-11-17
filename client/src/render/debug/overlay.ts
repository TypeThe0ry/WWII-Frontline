import type { PerfDump } from './perf-export';

/** DOM debug overlay. F3 toggles; `?debug=1` starts it visible. */
export class DebugOverlay {
  private readonly el: HTMLPreElement;
  private visible: boolean;
  private lastUpdate = 0;

  constructor(initiallyVisible: boolean, parent: HTMLElement = document.body) {
    this.visible = initiallyVisible;
    this.el = document.createElement('pre');
    this.el.id = 'debug-overlay';
    Object.assign(this.el.style, {
      position: 'fixed',
      left: '8px',
      top: '8px',
      margin: '0',
      padding: '6px 8px',
      font: '12px/1.35 ui-monospace, Menlo, Consolas, monospace',
      color: '#d7ffd7',
      background: 'rgba(0,0,0,0.55)',
      pointerEvents: 'none',
      whiteSpace: 'pre',
      zIndex: '10',
    } satisfies Partial<CSSStyleDeclaration>);
    this.el.style.display = initiallyVisible ? 'block' : 'none';
    parent.appendChild(this.el);
    window.addEventListener('keydown', (e) => {
      if (e.code === 'F3') {
        e.preventDefault();
        this.toggle();
      }
    });
  }

  toggle(): void {
    this.visible = !this.visible;
    this.el.style.display = this.visible ? 'block' : 'none';
  }

  get isVisible(): boolean {
    return this.visible;
  }

  /** Refresh text at most ~4 Hz. */
  update(now: number, d: PerfDump): void {
    if (!this.visible || now - this.lastUpdate < 250) return;
    this.lastUpdate = now;
    const f = (v: number) => v.toFixed(2);
    const gpu = d.gpu_ms.value === null ? 'n/a' : `${f(d.gpu_ms.value)} ms (${d.gpu_ms.source})`;
    const lines = [
      `renderer  ${d.renderer === 'webgpu' ? 'WebGPU' : 'WebGL2'}${d.fallback_reason ? `  [fallback: ${d.fallback_reason}]` : ''}`,
      `FPS       ${d.fps.toFixed(1)}`,
      `CPU ms    ${f(d.cpu_ms.mean)}  P50 ${f(d.cpu_ms.p50)}  P99 ${f(d.cpu_ms.p99)}  (n=${d.cpu_ms.count})`,
      `GPU ms    ${gpu}${d.gpu_ms.reason ? `  [${d.gpu_ms.reason}]` : ''}`,
      `draws     ${d.draw_calls}   meshes ${d.active_meshes}   tris ${d.triangles}`,
      `res       ${d.resolution.width}x${d.resolution.height}  render_scale ${d.render_scale.toFixed(2)}`,
      `sim       steps ${d.sim.steps_total}  clamped ${d.sim.stepper_clamped}`,
      `RTT ${d.slots.rtt}   transport ${d.slots.transport}   corrections ${d.slots.corrections}`,
    ];
    this.el.textContent = lines.join('\n');
  }
}
