#!/usr/bin/env node
// PI-117: checks the rendered ground shadows against a CPU ray-cast ground truth (building prisms, the scene's sun) at a grid of
// visible ground points, on WebGPU and WebGL2: agreement = fraction of points where "darker than the lit neighbourhood" matches
// "in shadow". Usage: node tools/perf/shadow-truth.mjs [--query "scene=world&..."] [--radius 70] [--step 3]
import { createRequire } from 'node:module';
import { mkdtempSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { decodePng } from './delta-e.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

/** Ray from a ground point towards the sun hits a building prism? (pure; used by the tests) */
export function inShadow(px, py, pz, sunDir, prisms, maxT = 120) {
  for (const b of prisms) {
    // march along the ray in 0.5 m steps (prisms are a few metres to tens of metres: fine for a ground-truth check)
    for (let t = 0.5; t < maxT; t += 0.5) {
      const x = px + sunDir[0] * t, y = py + sunDir[1] * t, z = pz + sunDir[2] * t;
      if (y > b.top) break;
      if (x < b.minX || x > b.maxX || z < b.minZ || z > b.maxZ) continue;
      if (y >= b.base && pointInPoly(x, z, b.ring)) return true;
    }
  }
  return false;
}

export function pointInPoly(x, z, ring) {
  let inside = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const [xi, zi] = ring[i], [xj, zj] = ring[j];
    if (zi > z !== zj > z && x < ((xj - xi) * (z - zi)) / (zj - zi) + xi) inside = !inside;
  }
  return inside;
}

async function main() {
  const a = process.argv.slice(2);
  const opt = (k, d) => (a.includes(k) ? a[a.indexOf(k) + 1] : d);
  const query = opt('--query', 'scene=world&map=carentan&cam=old_town&veg=0&actors=demo&entities=0&speed=0&csm=custom');
  const radius = Number(opt('--radius', 70)), step = Number(opt('--step', 3));
  const CLIENT = path.join(ROOT, 'client');
  const require = createRequire(path.join(CLIENT, 'package.json'));
  const { chromium } = require('@playwright/test');
  const { createServer } = require('vite');
  const cacheDir = mkdtempSync(path.join(os.tmpdir(), 'fl-st-vite-'));
  const vite = await createServer({ root: CLIENT, configFile: false, logLevel: 'error', cacheDir, optimizeDeps: { noDiscovery: true, include: [] }, server: { host: '127.0.0.1', port: 0, fs: { allow: [ROOT] }, hmr: false, watch: null } });
  await vite.listen();
  const port = vite.httpServer.address().port;
  const browser = await chromium.launch({ headless: true, args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan,WebGPU', '--use-angle=vulkan', '--ignore-gpu-blocklist'] });
  const out = {};
  try {
    for (const r of ['webgpu', 'webgl2']) {
      const page = await (await browser.newContext({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 })).newPage();
      await page.goto(`http://localhost:${port}/?renderer=${r}&${query}&debug=0`);
      await page.waitForFunction(() => window.__world && window.__terrain && (window.__perf?.frames() ?? 0) >= 20, undefined, { timeout: 120000 });
      await page.waitForTimeout(2500);
      const pts = await page.evaluate(async ({ radius, step }) => {
        const T = window.__terrain;
        const cam = window.__world.hedges ? null : null; void cam;
        const sc = window.__world.buildingMeshes[0].getScene();
        const c = sc.activeCamera.globalPosition;
        const sun = [-0.45, 0.8, -0.35];
        const L = Math.hypot(...sun);
        const toSun = sun.map((v) => v / L);
        const gj = await (await fetch('/maps/carentan/buildings.geojson')).json();
        const prisms = [];
        for (const f of gj.features) {
          const ring = f.geometry.coordinates[0];
          let minX = 1e9, maxX = -1e9, minZ = 1e9, maxZ = -1e9, lo = 1e9, hi = -1e9;
          for (const [x, z] of ring) { minX = Math.min(minX, x); maxX = Math.max(maxX, x); minZ = Math.min(minZ, z); maxZ = Math.max(maxZ, z); const g = T.heightAt(x, z); lo = Math.min(lo, g); hi = Math.max(hi, g); }
          if (maxX < c.x - radius - 80 || minX > c.x + radius + 80 || maxZ < c.z - radius - 80 || minZ > c.z + radius + 80) continue;
          prisms.push({ ring, minX, maxX, minZ, maxZ, base: lo - 0.4, top: hi + Math.max(1, f.properties.levels ?? 2) * 3 });
        }
        return { cam: [c.x, c.y, c.z], toSun, prisms };
      }, { radius, step });
      const { cam, toSun, prisms } = pts;
      const samples = await page.evaluate(({ cam, radius, step, toSun, prisms }) => {
        const T = window.__terrain; const res = [];
        const pip = (x, z, ring) => { let inside = false; for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) { const [xi, zi] = ring[i], [xj, zj] = ring[j]; if (zi > z !== zj > z && x < ((xj - xi) * (z - zi)) / (zj - zi) + xi) inside = !inside; } return inside; };
        const hitPrism = (px, py, pz, d, maxT) => { for (const b of prisms) { for (let t = 0.5; t < maxT; t += 0.5) { const x = px + d[0] * t, y = py + d[1] * t, z = pz + d[2] * t; if (y > b.top) break; if (x < b.minX || x > b.maxX || z < b.minZ || z > b.maxZ) continue; if (y >= b.base && pip(x, z, b.ring)) return true; } } return false; };
        for (let dx = -radius; dx <= radius; dx += step) for (let dz = -radius; dz <= radius; dz += step) {
          const x = cam[0] + dx, z = cam[2] + dz; const y = T.heightAt(x, z);
          const p = T.project(x, z); if (!(p.depth > 0 && p.depth < 1)) continue;
          // visible: the ray from the camera to the point is not blocked by a prism
          const vd = [x - cam[0], y - cam[1], z - cam[2]]; const vl = Math.hypot(...vd); const vdir = vd.map((v) => v / vl);
          if (hitPrism(cam[0], cam[1], cam[2], vdir, vl - 0.6)) continue;
          // not inside a building footprint
          if (prisms.some((b) => x >= b.minX && x <= b.maxX && z >= b.minZ && z <= b.maxZ && pip(x, z, b.ring))) continue;
          res.push({ x, z, sx: p.x, sy: p.y, shadow: hitPrism(x, y + 0.05, z, toSun, 120) });
        }
        return res;
      }, { cam, radius, step, toSun, prisms });
      const png = decodePng(await page.screenshot());
      const lum = (sx, sy) => { const i = (Math.round(sy) * png.w + Math.round(sx)) * 4; return 0.2126 * png.rgba[i] + 0.7152 * png.rgba[i + 1] + 0.0722 * png.rgba[i + 2]; };
      const inView = samples.filter((s) => s.sx > 4 && s.sy > 4 && s.sx < png.w - 5 && s.sy < png.h - 5);
      // rendered "dark" = luminance below 55 % of the median luminance of the lit points
      const litL = inView.filter((s) => !s.shadow).map((s) => lum(s.sx, s.sy)).sort((p, q) => p - q);
      const med = litL[litL.length >> 1] ?? 1;
      let agree = 0, tp = 0, fp = 0, fn = 0, tn = 0;
      for (const s of inView) {
        const dark = lum(s.sx, s.sy) < 0.55 * med;
        if (dark === s.shadow) agree++;
        if (s.shadow && dark) tp++; else if (!s.shadow && dark) fp++; else if (s.shadow && !dark) fn++; else tn++;
      }
      out[r] = { points: inView.length, shadow_truth: inView.filter((s) => s.shadow).length, agreement: agree / Math.max(1, inView.length), tp, fp, fn, tn };
      await page.close();
    }
  } finally {
    await browser.close();
    await vite.close();
    rmSync(cacheDir, { recursive: true, force: true });
  }
  console.log(JSON.stringify(out, null, 1));
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main().catch((e) => { console.error(e); process.exit(1); });
