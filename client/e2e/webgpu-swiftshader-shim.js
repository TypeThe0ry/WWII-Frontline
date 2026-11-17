// Test-only shim for headless Chromium + SwiftShader WebGPU (no GPU, CI/dev containers).
// Compute and render work on the software adapter, but presenting to a canvas
// (GPUCanvasContext.getCurrentTexture) kills the Dawn instance there. This shim hands out an
// off-screen texture instead, so Babylon's WebGPUEngine can run its whole pipeline (init, render,
// compute, timestamp queries) without presenting. Pixel output of the canvas is NOT visible.
// Injected with page.addInitScript(); see render-bootstrap.spec.ts.
(() => {
  if (!('GPUCanvasContext' in window)) return;
  const proto = GPUCanvasContext.prototype;
  const origConfigure = proto.configure;
  const state = new WeakMap();
  proto.configure = function (cfg) {
    state.set(this, { cfg, tex: null });
    // Do not call the real configure: any real swapchain use is what breaks on SwiftShader.
  };
  proto.getCurrentTexture = function () {
    const s = state.get(this);
    if (!s) throw new Error('shim: getCurrentTexture before configure');
    const w = this.canvas.width || 1;
    const h = this.canvas.height || 1;
    if (!s.tex || s.tex.width !== w || s.tex.height !== h) {
      s.tex?.destroy();
      s.tex = s.cfg.device.createTexture({
        size: [w, h],
        format: s.cfg.format,
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC | GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
      });
    }
    return s.tex;
  };
  proto.unconfigure = function () {
    state.delete(this);
  };
  void origConfigure;
})();
