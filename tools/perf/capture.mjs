#!/usr/bin/env node
// M0-030 client performance capture: Chromium + the real client at ?scene=vegetation&cam=N&debug=1, warm-up, reset the
// statistics windows, capture, write window.__perf.dump() (+ meta) per run. One command, all cameras:
//
//   node tools/perf/capture.mjs                                   # software rendering (SwiftShader), WebGL2, 3 cams x 3 runs x 30 s
//   node tools/perf/capture.mjs --gpu --renderer webgpu --viewport 1920x1080 --machine rtx3060   # reference machine
//   node tools/perf/capture.mjs --duration 5 --warmup 2 --repeats 1 --cams 0   # smoke
//
// Options: --renderer webgl2|webgpu (default webgl2)  --cams 0,1,2  --repeats N (3)  --warmup S (5)  --duration S (30)
//          --viewport WxH (640x360 without --gpu, 1920x1080 with)  --quality high|low  --machine NAME (default: hostname)
//          --out-dir DIR (reports/m0/perf)  --gpu (use the machine's GPU: no SwiftShader flags)  --headed
//          --url URL (an already running client instead of starting a Vite dev server)
// Output: <out-dir>/perf-<machine>-<renderer>-cam<N>.json  { meta, runs: [PerfDump + meta], aggregate }
// Without a usable WebGPU adapter a webgpu request is skipped and written as { skipped: reason } (reported "未测").
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { aggregateRuns } from './lib.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const CLIENT = path.join(ROOT, 'client');
const require = createRequire(path.join(CLIENT, 'package.json'));

function parseArgs(argv) {
  const o = { renderer: 'webgl2', cams: '0,1,2', repeats: 3, warmup: 5, duration: 30, viewport: null, quality: 'high', machine: os.hostname(), outDir: path.join(ROOT, 'reports/m0/perf'), gpu: false, headed: false, url: null };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i];
    if (a === '--renderer') o.renderer = next();
    else if (a === '--cams') o.cams = next();
    else if (a === '--repeats') o.repeats = Number(next());
    else if (a === '--warmup') o.warmup = Number(next());
    else if (a === '--duration') o.duration = Number(next());
    else if (a === '--viewport') o.viewport = next();
    else if (a === '--quality') o.quality = next();
    else if (a === '--machine') o.machine = next();
    else if (a === '--out-dir') o.outDir = path.resolve(next());
    else if (a === '--gpu') o.gpu = true;
    else if (a === '--headed') o.headed = true;
    else if (a === '--url') o.url = next();
    else if (a === '-h' || a === '--help') {
      console.log(readHelp());
      process.exit(0);
    } else throw new Error(`unknown option ${a}`);
  }
  if (!['webgl2', 'webgpu'].includes(o.renderer)) throw new Error('--renderer must be webgl2 or webgpu');
  o.viewport ??= o.gpu ? '1920x1080' : '640x360';
  const m = /^(\d+)x(\d+)$/.exec(o.viewport);
  if (!m) throw new Error('--viewport must be WxH');
  o.size = { width: Number(m[1]), height: Number(m[2]) };
  o.camList = o.cams.split(',').map(Number);
  return o;
}

function readHelp() {
  return 'See the header of tools/perf/capture.mjs and tools/perf/README.md.';
}

function git(...args) {
  try {
    return execFileSync('git', args, { cwd: ROOT, encoding: 'utf8' }).trim();
  } catch {
    return 'unknown';
  }
}

async function startVite() {
  const { createServer } = require('vite');
  const cacheDir = mkdtempSync(path.join(os.tmpdir(), 'fl-perf-vite-'));
  const vite = await createServer({
    root: CLIENT,
    configFile: false,
    logLevel: 'error',
    cacheDir,
    optimizeDeps: { noDiscovery: true, include: [] },
    server: { host: '127.0.0.1', port: 0, strictPort: false, fs: { allow: [ROOT] }, hmr: false, watch: null },
  });
  await vite.listen();
  const addr = vite.httpServer?.address();
  if (!addr || typeof addr === 'string') throw new Error('vite has no address');
  return { url: `http://localhost:${addr.port}/`, close: async () => { await vite.close(); rmSync(cacheDir, { recursive: true, force: true }); } };
}

function chromiumArgs(o) {
  const args = ['--enable-precise-memory-info', '--disable-background-timer-throttling', '--disable-renderer-backgrounding', '--disable-backgrounding-occluded-windows'];
  if (!o.gpu) args.push('--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist');
  if (o.renderer === 'webgpu') {
    args.push('--enable-unsafe-webgpu', '--enable-features=Vulkan,WebGPU');
    if (!o.gpu) args.push('--use-webgpu-adapter=swiftshader');
  }
  return args;
}

async function runOnce(browser, o, baseUrl, cam, runIndex) {
  const context = await browser.newContext({ viewport: o.size, deviceScaleFactor: 1 });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`));
  if (o.renderer === 'webgpu' && !o.gpu) await page.addInitScript({ path: path.join(CLIENT, 'e2e/webgpu-swiftshader-shim.js') });
  const q = new URLSearchParams({ renderer: o.renderer, scene: 'vegetation', cam: String(cam), debug: '1', quality: o.quality });
  const t0 = Date.now();
  await page.goto(`${baseUrl}?${q}`);
  await page.waitForFunction(() => window.__vegetation !== undefined && window.__terrain !== undefined && (window.__perf?.frames() ?? 0) >= 2, undefined, { timeout: 180_000 });
  const loadMs = Date.now() - t0;
  await page.waitForTimeout(o.warmup * 1000);
  await page.evaluate(() => window.__perf.reset?.());
  const c0 = Date.now();
  await page.waitForTimeout(o.duration * 1000);
  const captured = (Date.now() - c0) / 1000;
  const dump = await page.evaluate(() => {
    const d = window.__perf.dump();
    const mem = performance.memory;
    const caps = window.__caps ?? {};
    return { dump: d, heap: mem ? mem.usedJSHeapSize / 1048576 : null, caps: { kind: caps.kind, fallbackReason: caps.fallbackReason, gpuTimerSource: caps.gpuTimerSource } };
  });
  await context.close();
  return {
    ...dump.dump,
    meta: {
      cam,
      run: runIndex,
      load_ms: loadMs,
      warmup_s: o.warmup,
      captured_s: captured,
      js_heap_mb: dump.heap,
      page_errors: errors,
      caps: dump.caps,
    },
  };
}

async function main() {
  const o = parseArgs(process.argv.slice(2));
  mkdirSync(o.outDir, { recursive: true });
  const { chromium } = require('playwright');
  const server = o.url ? { url: o.url.endsWith('/') ? o.url : `${o.url}/`, close: async () => {} } : await startVite();
  const browser = await chromium.launch({ headless: !o.headed, args: chromiumArgs(o) });
  const meta = {
    machine: o.machine,
    cpu: os.cpus()[0]?.model ?? 'unknown',
    cores: os.cpus().length,
    mem_gb: +(os.totalmem() / 2 ** 30).toFixed(1),
    os: `${os.type()} ${os.release()}`,
    node: process.version,
    chromium: browser.version(),
    gpu_mode: o.gpu ? 'hardware' : 'swiftshader (software)',
    renderer_requested: o.renderer,
    viewport: o.viewport,
    quality: o.quality,
    commit: git('rev-parse', '--short', 'HEAD'),
    dirty: git('status', '--porcelain').length > 0,
    date: new Date().toISOString(),
    command: `node tools/perf/capture.mjs ${process.argv.slice(2).join(' ')}`.trim(),
  };
  console.log(`[perf] ${meta.machine}: ${meta.cpu} x${meta.cores}, Chromium ${meta.chromium}, ${meta.gpu_mode}, ${o.renderer} ${o.viewport}`);
  let exit = 0;
  try {
    for (const cam of o.camList) {
      const file = path.join(o.outDir, `perf-${o.machine}-${o.renderer}-cam${cam}.json`);
      const runs = [];
      let skipped = null;
      for (let r = 0; r < o.repeats; r++) {
        try {
          const d = await runOnce(browser, o, server.url, cam, r);
          if (d.renderer !== o.renderer) {
            skipped = `requested ${o.renderer} but the page ran ${d.renderer} (${d.fallback_reason ?? 'no reason reported'})`;
            break;
          }
          runs.push(d);
          console.log(`[perf] cam ${cam} run ${r + 1}/${o.repeats}: ${d.frames} frames, fps ${d.fps.toFixed(2)}, cpu p50 ${d.cpu_ms.p50.toFixed(1)} ms, draw calls ${d.draw_calls}, tris ${d.triangles}, veg instances ${d.vegetation.instances}`);
        } catch (e) {
          skipped = `run failed: ${e instanceof Error ? e.message.split('\n')[0] : String(e)}`;
          break;
        }
      }
      const out = skipped && runs.length === 0 ? { meta, cam, skipped, 未测: skipped } : { meta, cam, runs, aggregate: aggregateRuns(runs), ...(skipped ? { partial: skipped } : {}) };
      writeFileSync(file, JSON.stringify(out, null, 2) + '\n');
      console.log(`[perf] wrote ${path.relative(ROOT, file)}${skipped ? ` (${skipped})` : ''}`);
      if (skipped && runs.length === 0) exit = exit || 3;
    }
  } finally {
    await browser.close();
    await server.close();
  }
  process.exit(exit);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((e) => {
    console.error(`[perf] ${e instanceof Error ? e.stack : e}`);
    process.exit(1);
  });
}
