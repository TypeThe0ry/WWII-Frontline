#!/usr/bin/env node
// M1-042b: WGSL vs GLSL visual regression. Renders `?scene=pbr` with WebGPU and with WebGL2 (real GPU), decodes both
// screenshots and reports the CIE76 delta E between them. Usage:
//   node tools/perf/delta-e.mjs [--out reports/m1-pbr-delta-e.json] [--keep DIR] [--query "scene=world&map=carentan&cam=old_town&..."] [--wait world|pbr]
// Default: `?scene=pbr` (M1-042b). With --query any page can be compared (e.g. the CSM scene, M1-043b); keep it free of time-dependent content.
import { createRequire } from 'node:module';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { inflateSync } from 'node:zlib';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

/** sRGB 0..255 -> CIE L*a*b* (D65). */
export function srgbToLab(r, g, b) {
  const lin = (v) => {
    const c = v / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  const R = lin(r), G = lin(g), B = lin(b);
  const X = (0.4124564 * R + 0.3575761 * G + 0.1804375 * B) / 0.95047;
  const Y = 0.2126729 * R + 0.7151522 * G + 0.072175 * B;
  const Z = (0.0193339 * R + 0.119192 * G + 0.9503041 * B) / 1.08883;
  const f = (t) => (t > 216 / 24389 ? Math.cbrt(t) : (24389 / 27 * t + 16) / 116);
  const fx = f(X), fy = f(Y), fz = f(Z);
  return [116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz)];
}

export const deltaE76 = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);

/** Minimal PNG decoder for 8-bit non-interlaced RGB/RGBA (what Playwright screenshots are). Returns {w, h, rgba}. */
export function decodePng(buf) {
  if (buf.readUInt32BE(0) !== 0x89504e47) throw new Error('not a PNG');
  let off = 8, w = 0, h = 0, ct = 0;
  const idat = [];
  while (off < buf.length) {
    const len = buf.readUInt32BE(off), type = buf.toString('ascii', off + 4, off + 8), data = buf.subarray(off + 8, off + 8 + len);
    if (type === 'IHDR') {
      w = data.readUInt32BE(0); h = data.readUInt32BE(4);
      if (data[8] !== 8 || data[12] !== 0) throw new Error('only 8-bit non-interlaced PNG');
      ct = data[9];
    } else if (type === 'IDAT') idat.push(data);
    else if (type === 'IEND') break;
    off += 12 + len;
  }
  const bpp = ct === 6 ? 4 : ct === 2 ? 3 : 0;
  if (!bpp) throw new Error(`unsupported colour type ${ct}`);
  const raw = inflateSync(Buffer.concat(idat));
  const stride = w * bpp;
  const out = Buffer.alloc(w * h * 4);
  let prev = Buffer.alloc(stride), pos = 0;
  for (let y = 0; y < h; y++) {
    const ft = raw[pos++];
    const line = Buffer.from(raw.subarray(pos, pos + stride));
    pos += stride;
    for (let i = 0; i < stride; i++) {
      const a = i >= bpp ? line[i - bpp] : 0, b = prev[i], c = i >= bpp ? prev[i - bpp] : 0;
      let v = line[i];
      if (ft === 1) v += a;
      else if (ft === 2) v += b;
      else if (ft === 3) v += (a + b) >> 1;
      else if (ft === 4) {
        const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
        v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      }
      line[i] = v & 255;
    }
    for (let x = 0; x < w; x++) {
      out[(y * w + x) * 4] = line[x * bpp]; out[(y * w + x) * 4 + 1] = line[x * bpp + 1]; out[(y * w + x) * 4 + 2] = line[x * bpp + 2]; out[(y * w + x) * 4 + 3] = bpp === 4 ? line[x * bpp + 3] : 255;
    }
    prev = line;
  }
  return { w, h, rgba: out };
}

/** Per-pixel delta E statistics between two equally sized RGBA images. */
export function compareImages(a, b) {
  if (a.w !== b.w || a.h !== b.h) throw new Error('size mismatch');
  const n = a.w * a.h;
  const d = new Float64Array(n);
  for (let i = 0; i < n; i++) d[i] = deltaE76(srgbToLab(a.rgba[i * 4], a.rgba[i * 4 + 1], a.rgba[i * 4 + 2]), srgbToLab(b.rgba[i * 4], b.rgba[i * 4 + 1], b.rgba[i * 4 + 2]));
  const s = [...d].sort((x, y) => x - y);
  const mean = d.reduce((x, y) => x + y, 0) / n;
  return { pixels: n, mean, p99: s[Math.floor(0.99 * (n - 1))], p9999: s[Math.floor(0.9999 * (n - 1))], max: s[n - 1], over1: d.filter((v) => v > 1).length / n, over2: d.filter((v) => v > 2).length / n, over5: d.filter((v) => v > 5).length / n };
}

async function main() {
  const a = process.argv.slice(2);
  const opt = (k, d) => (a.includes(k) ? a[a.indexOf(k) + 1] : d);
  const out = path.resolve(ROOT, opt('--out', 'reports/m1-pbr-delta-e.json'));
  const keep = opt('--keep', null);
  const query = opt('--query', 'scene=pbr');
  const waitFor = opt('--wait', query.includes('scene=world') ? 'world' : 'pbr');
  const CLIENT = path.join(ROOT, 'client');
  const require = createRequire(path.join(CLIENT, 'package.json'));
  const { chromium } = require('@playwright/test');
  const { createServer } = require('vite');
  const cacheDir = mkdtempSync(path.join(os.tmpdir(), 'fl-de-vite-'));
  const vite = await createServer({ root: CLIENT, configFile: false, logLevel: 'error', cacheDir, optimizeDeps: { noDiscovery: true, include: [] }, server: { host: '127.0.0.1', port: 0, fs: { allow: [ROOT] }, hmr: false, watch: null } });
  await vite.listen();
  const port = vite.httpServer.address().port;
  const browser = await chromium.launch({ headless: true, args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan,WebGPU', '--use-angle=vulkan', '--ignore-gpu-blocklist'] });
  const shots = {}, meta = {};
  try {
    for (const r of ['webgpu', 'webgl2']) {
      const page = await (await browser.newContext({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 })).newPage();
      await page.goto(`http://localhost:${port}/?renderer=${r}&${query}`);
      await page.waitForFunction((k) => window[`__${k}`] && (window.__perf?.frames() ?? 0) >= 10, waitFor, { timeout: 120000 });
      await page.waitForTimeout(1500);
      meta[r] = await page.evaluate(() => ({ renderer: window.__perf.dump().renderer, prewarm: window.__pbr ? { variants: window.__pbr.prewarm.variants, total_ms: window.__pbr.prewarm.totalMs, max_ms: window.__pbr.prewarm.maxMs, failed: window.__pbr.prewarm.failed } : null }));
      shots[r] = await page.screenshot();
      if (keep) { mkdirSync(keep, { recursive: true }); writeFileSync(path.join(keep, `pbr-${r}.png`), shots[r]); }
    }
  } finally {
    await browser.close();
    await vite.close();
    rmSync(cacheDir, { recursive: true, force: true });
  }
  if (meta.webgpu.renderer !== 'webgpu' || meta.webgl2.renderer !== 'webgl2') throw new Error(`unexpected renderers ${JSON.stringify(meta)}`);
  const stats = compareImages(decodePng(shots.webgpu), decodePng(shots.webgl2));
  const doc = { task: query === 'scene=pbr' ? 'M1-042b' : 'M1-043b', date: new Date().toISOString(), scene: query === 'scene=pbr' ? '?scene=pbr (10 spheres metallic x roughness, normal-mapped + detail plane, alpha-tested plane, blended plane)' : `?${query}`, viewport: '1280x720', metric: 'CIE76 delta E, WebGPU (WGSL) vs WebGL2 (GLSL) screenshots', stats, meta };
  writeFileSync(out, JSON.stringify(doc, null, 2) + '\n');
  console.log(JSON.stringify({ stats, prewarm: { webgpu: meta.webgpu.prewarm, webgl2: meta.webgl2.prewarm } }, null, 1));
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main().catch((e) => { console.error(e); process.exit(1); });
