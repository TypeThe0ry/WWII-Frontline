#!/usr/bin/env node
// M1-044b / PI-118: TAA ghosting measurement. Renders `?scene=taa` (static camera, a box moving 3 px / loop frame, deterministic frame index)
// for N frames with a given post configuration and compares the last frame with the same frame rendered through the post pipeline with every effect off:
//   trail  = mean |dL| over the band the box crossed in the last 14 frames (excluding the box itself): ghosting shows up here
//   outside = mean |dL| over the rest of the image (anti-aliasing / tone-mapping differences, the noise floor of the comparison)
// Usage: node tools/perf/taa-ghost.mjs [--frames 60] [--out FILE]
import { createRequire } from 'node:module';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { decodePng, srgbToLab } from './delta-e.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

/** Mean |dL| of two images over `inside(x, y)`; pure, tested. */
export function meanDL(a, b, inside) {
  let s = 0, n = 0;
  for (let y = 0; y < a.h; y++) for (let x = 0; x < a.w; x++) {
    if (!inside(x, y)) continue;
    const i = (y * a.w + x) * 4;
    s += Math.abs(srgbToLab(a.rgba[i], a.rgba[i + 1], a.rgba[i + 2])[0] - srgbToLab(b.rgba[i], b.rgba[i + 1], b.rgba[i + 2])[0]);
    n++;
  }
  return n ? s / n : 0;
}

async function main() {
  const args = process.argv.slice(2);
  const opt = (k, d) => (args.includes(k) ? args[args.indexOf(k) + 1] : d);
  const frames = Number(opt('--frames', 60));
  const CLIENT = path.join(ROOT, 'client');
  const require = createRequire(path.join(CLIENT, 'package.json'));
  const { chromium } = require('@playwright/test');
  const { createServer } = require('vite');
  const cacheDir = mkdtempSync(path.join(os.tmpdir(), 'fl-taa-vite-'));
  const vite = await createServer({ root: CLIENT, configFile: false, logLevel: 'error', cacheDir, optimizeDeps: { noDiscovery: true, include: [] }, server: { host: '127.0.0.1', port: 0, fs: { allow: [ROOT] }, hmr: false, watch: null } });
  await vite.listen();
  const port = vite.httpServer.address().port;
  const browser = await chromium.launch({ headless: true, args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan,WebGPU', '--use-angle=vulkan', '--ignore-gpu-blocklist'] });
  const configs = [
    // the reference goes through the same post pipeline (colour space, image processing) with every effect off: AA and TAA are the only differences
    ['no_post', 'post=low&fxaa=0&bloom=0&tonemap=0&post_guard=0'],
    ['fxaa_only', 'post=low&bloom=0&tonemap=0'],
    ['taa_no_clamp', 'post=high&bloom=0&tonemap=0&taa_clamp=0&post_guard=0'],
    ['taa_clamp', 'post=high&bloom=0&tonemap=0&taa_clamp=1&post_guard=0'],
    ['taa_clamp_factor08', 'post=high&bloom=0&tonemap=0&taa_clamp=1&taa_factor=0.8&post_guard=0'],
    ['taa_clamp_factor095', 'post=high&bloom=0&tonemap=0&taa_clamp=1&taa_factor=0.95&post_guard=0'],
    ['taa_clamp_factor05', 'post=high&bloom=0&tonemap=0&taa_clamp=1&taa_factor=0.5&post_guard=0'],
  ];
  const out = { frames, renderers: {} };
  try {
    for (const r of ['webgpu', 'webgl2']) {
      const shots = {};
      let box = null;
      for (const [name, q] of configs) {
        const page = await (await browser.newContext({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 })).newPage();
        await page.goto(`http://localhost:${port}/?renderer=${r}&scene=taa&debug=0${q ? '&' + q : ''}`);
        await page.waitForFunction(() => window.__taa !== undefined, undefined, { timeout: 120000 });
        const b = await page.evaluate((n) => window.__taa.runTo(n), frames);
        await page.waitForTimeout(300);
        box = box ?? b;
        shots[name] = decodePng(await page.screenshot());
        await page.close();
      }
      const ref = shots.no_post;
      const stepPx = 3.4; // measured below from two positions
      const x1 = box.x, y = box.y, halfW = box.w / 2, halfH = box.h / 2;
      const inTrail = (x, yy) => yy > y - halfH && yy < y + halfH && x < x1 - halfW - 2 && x > x1 - halfW - 14 * stepPx;
      const inBox = (x, yy) => Math.abs(x - x1) < halfW + 4 && Math.abs(yy - y) < halfH + 4;
      out.renderers[r] = { box, configs: {} };
      for (const [name] of configs) {
        if (name === 'no_post') continue;
        out.renderers[r].configs[name] = {
          trail_dL: meanDL(shots[name], ref, inTrail),
          outside_dL: meanDL(shots[name], ref, (x, yy) => !inTrail(x, yy) && !inBox(x, yy)),
        };
      }
    }
  } finally {
    await browser.close();
    await vite.close();
    rmSync(cacheDir, { recursive: true, force: true });
  }
  const f = opt('--out', null);
  if (f) writeFileSync(f, JSON.stringify(out, null, 2) + '\n');
  console.log(JSON.stringify(out, null, 1));
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main().catch((e) => { console.error(e); process.exit(1); });
